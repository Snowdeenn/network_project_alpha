pub(super) mod r#struct;

use r#struct::VfxEffectDescriptor;

pub(crate) fn parse_effect(source: &str) -> Result<VfxEffectDescriptor, std::io::Error> {
    let parsed_effect: VfxEffectDescriptor = serde_json::from_str(source)?;
    Ok(parsed_effect)
}
