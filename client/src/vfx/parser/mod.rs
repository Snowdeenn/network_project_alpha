pub(super) mod r#struct;

use r#struct::VfxEffectDescriptor;

pub(crate) fn parse_effect(source: &str) -> Result<VfxEffectDescriptor, std::io::Error> {
    super::validation::validate_json(source)?;
    let parsed_effect: VfxEffectDescriptor = serde_json::from_str(source)?;
    super::validation::validate_descriptor(&parsed_effect)?;
    Ok(parsed_effect)
}
