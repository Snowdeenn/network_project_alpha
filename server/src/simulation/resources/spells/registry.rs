use utils::ids::SpellTag;
use utils::spell_types::{RawSpell, Spell, SpellId};

pub struct SpellRegister {
    inner: utils::Arena<Spell, SpellTag>,
    string_to_id: std::collections::HashMap<String, SpellId>,
}

impl SpellRegister {
    pub fn init(file_path: &str) -> std::io::Result<Self> {
        tracing::info!(path = %file_path, "Chargement du registre de sorts...");

        let spell_config_file = std::fs::read(file_path).map_err(|e| {
        tracing::error!(path = %file_path, error = %e, "Impossible de lire le fichier de sorts");
        e
    })?;

        let raw_spells: Vec<RawSpell> =
            serde_json::from_slice(&spell_config_file).map_err(|e| {
                tracing::error!(path = %file_path, error = %e, "Erreur de désérialisation JSON");
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;

        let mut inner = utils::Arena::new();
        let mut string_to_id = std::collections::HashMap::new();

        for raw_spell in raw_spells {
            let (raw_spell_id, spell) = raw_spell.into_spell();
            validate_spell(&raw_spell_id, &spell)?;
            if string_to_id.contains_key(&raw_spell_id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Identifiant de sort dupliqué : {raw_spell_id}"),
                ));
            }
            let spell_arena_id = inner.insert(spell);
            string_to_id.insert(raw_spell_id, SpellId::from(spell_arena_id));
        }

        tracing::info!("Registre de sorts initialisé avec succès");

        Ok(Self {
            inner,
            string_to_id,
        })
    }

    pub fn resolve_string(&self, str: &str) -> Option<&SpellId> {
        self.string_to_id.get(str)
    }

    pub fn get_spell(&self, spell_id: SpellId) -> Option<&Spell> {
        self.inner.get(*spell_id)
    }

    pub fn all_ids(&self) -> Vec<SpellId> {
        self.inner
            .iter_with_ids()
            .map(|(id, _)| SpellId::from(id))
            .collect()
    }
}

fn validate_spell(id: &str, spell: &Spell) -> std::io::Result<()> {
    let invalid = !spell.cast_cost.cooldown.is_finite()
        || spell.cast_cost.cooldown < 0.0
        || !spell.targeting.range.is_finite()
        || spell.targeting.range < 0.0
        || !spell.targeting.projectile_radius.is_finite()
        || spell.targeting.projectile_radius < 0.0
        || !spell.targeting.speed.is_finite()
        || (matches!(
            spell.targeting.kind,
            utils::spell_types::SpellTargetingKind::Directional
        ) && spell.targeting.speed <= 0.0);
    if invalid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Configuration invalide pour le sort {id}"),
        ));
    }
    Ok(())
}
