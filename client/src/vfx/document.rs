//! Shared parsing for elementary effects and composites, including async loads.
use super::parser::{parse_effect, r#struct::VfxEffectDescriptor};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositeComponent {
    pub effect: String,
    pub delay_seconds: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositeDocument {
    pub version: u32,
    pub name: String,
    pub family: String,
    pub components: Vec<CompositeComponent>,
}

#[derive(Debug)]
pub enum VfxDocument {
    Effect(VfxEffectDescriptor),
    Composite(CompositeDocument),
}

impl VfxDocument {
    pub fn parse(source: &str) -> std::io::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(source)?;
        if value.get("family").and_then(|v| v.as_str()) == Some("composite") {
            super::validation::validate_json(source)?;
            Ok(Self::Composite(serde_json::from_value(value)?))
        } else {
            Ok(Self::Effect(parse_effect(source)?))
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Effect(effect) => &effect.name,
            Self::Composite(effect) => &effect.name,
        }
    }

    pub fn dependencies(&self) -> Vec<String> {
        match self {
            Self::Effect(_) => Vec::new(),
            Self::Composite(effect) => effect.components.iter().map(|c| c.effect.clone()).collect(),
        }
    }
}
