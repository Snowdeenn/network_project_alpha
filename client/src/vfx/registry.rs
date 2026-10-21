//! Résolution par nom et publication de révisions immuables.
//! Le JSON et AssetRegister existants restent la source des descriptions.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

use utils::Id;

use super::parser::parse_effect;
use super::parser::r#struct::VfxEffectDescriptor;
use super::storage::AssetRegister;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    Essential,
}

pub(super) enum DefinitionKind {
    Effect(Id<VfxEffectDescriptor>),
    Composite(Vec<(Arc<VfxDefinition>, f32)>),
}

pub struct VfxDefinition {
    pub name: String,
    pub revision: u64,
    pub priority: Priority,
    pub(super) kind: DefinitionKind,
}

pub struct VfxRegistry {
    pub(super) assets: AssetRegister,
    definitions: HashMap<String, Arc<VfxDefinition>>,
    retired: Vec<Arc<VfxDefinition>>,
}

impl Default for VfxRegistry {
    fn default() -> Self {
        Self {
            assets: AssetRegister::new(),
            definitions: HashMap::new(),
            retired: Vec::new(),
        }
    }
}

impl VfxRegistry {
    pub fn get(&self, name: &str) -> Option<&Arc<VfxDefinition>> {
        self.definitions.get(name)
    }

    /// Un échec ne publie rien : l'ancienne révision reste utilisable.
    pub fn load(&mut self, path: impl AsRef<Path>, priority: Priority) -> io::Result<()> {
        let path = path.as_ref();
        let result =
            std::fs::read_to_string(path).and_then(|source| self.register_json(&source, priority));
        result.map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    }

    pub fn register_json(&mut self, source: &str, priority: Priority) -> io::Result<()> {
        super::validation::validate_json(source)?;
        let document: serde_json::Value = serde_json::from_str(source)?;
        if document.get("family").and_then(|family| family.as_str()) == Some("composite") {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Component {
                effect: String,
                delay_seconds: f32,
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Composite {
                version: u32,
                name: String,
                family: String,
                components: Vec<Component>,
            }
            let composite: Composite = serde_json::from_value(document)?;
            if composite.version != 1 || composite.family != "composite" {
                return Err(invalid("composite: expected version 1"));
            }
            let children: Vec<_> = composite
                .components
                .iter()
                .map(|c| (c.effect.as_str(), c.delay_seconds))
                .collect();
            return self.register_composite(&composite.name, &children, priority);
        }
        let descriptor = parse_effect(source)?;
        super::validation::validate_descriptor(&descriptor)?;
        let name = descriptor.name.clone();
        let descriptor = self.assets.insert(descriptor);
        self.publish(name, priority, DefinitionKind::Effect(descriptor));
        Ok(())
    }

    /// Composition de descriptions déjà chargées ; délais en secondes.
    /// Les références sont figées dans la révision, comme celles des instances.
    pub fn register_composite(
        &mut self,
        name: &str,
        children: &[(&str, f32)],
        priority: Priority,
    ) -> io::Result<()> {
        if name.trim().is_empty() || children.is_empty() || children.len() > 64 {
            return Err(invalid("composite: expected a name and 1..=64 children"));
        }
        let mut components = Vec::new();
        let mut count = 0;
        for &(child_name, delay) in children {
            if !delay.is_finite() || !(0.0..=60.0).contains(&delay) {
                return Err(invalid("composite: invalid delay"));
            }
            let child = self
                .get(child_name)
                .ok_or_else(|| invalid(format!("unknown child: {child_name}")))?;
            validate_child(child, name, 1, &mut count)?;
            components.push((Arc::clone(child), delay));
        }
        self.publish(
            name.to_owned(),
            priority,
            DefinitionKind::Composite(components),
        );
        Ok(())
    }

    fn publish(&mut self, name: String, priority: Priority, kind: DefinitionKind) {
        let revision = self
            .get(&name)
            .map_or(1, |definition| definition.revision + 1);
        let definition = Arc::new(VfxDefinition {
            name: name.clone(),
            revision,
            priority,
            kind,
        });
        if let Some(previous) = self.definitions.insert(name, definition) {
            self.retired.push(previous);
        }
    }

    pub(super) fn collect_retired(&mut self) {
        // Une ancienne définition composite peut encore retenir ses enfants.
        let mut index = 0;
        while index < self.retired.len() {
            if Arc::strong_count(&self.retired[index]) == 1 {
                let definition = self.retired.swap_remove(index);
                if let DefinitionKind::Effect(id) = &definition.kind {
                    self.assets.remove(*id);
                }
            } else {
                index += 1;
            }
        }
    }
}

fn validate_child(
    child: &VfxDefinition,
    parent: &str,
    depth: usize,
    count: &mut usize,
) -> io::Result<()> {
    if child.name == parent || depth > 8 {
        return Err(invalid("composite: cycle or depth greater than 8"));
    }
    match &child.kind {
        DefinitionKind::Effect(_) => *count += 1,
        DefinitionKind::Composite(children) => {
            for (child, _) in children {
                validate_child(child, parent, depth + 1, count)?;
            }
        }
    }
    if *count > 64 {
        return Err(invalid("composite: more than 64 descendants"));
    }
    Ok(())
}

pub(super) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
