use super::*;

const PARTICLE: &str = include_str!("../../../../../assets/vfx/exemples_vfx/particle.json");

#[test]
fn deserialize_all_example_families() {
    let examples = [
        ("particle", PARTICLE),
        (
            "mesh",
            include_str!("../../../../../assets/vfx/exemples_vfx/mesh.json"),
        ),
        (
            "trail",
            include_str!("../../../../../assets/vfx/exemples_vfx/trail.json"),
        ),
        (
            "decal",
            include_str!("../../../../../assets/vfx/exemples_vfx/decal.json"),
        ),
        (
            "light",
            include_str!("../../../../../assets/vfx/exemples_vfx/light.json"),
        ),
        (
            "post-process",
            include_str!("../../../../../assets/vfx/exemples_vfx/post-process.json"),
        ),
    ];
    for (expected, json) in examples {
        let effect: VfxEffectDescriptor = serde_json::from_str(json).unwrap();
        assert_eq!(effect.version, 1);
        assert!(!effect.name.is_empty());
        let family = match effect.family {
            VfxFamily::Particle { .. } => "particle",
            VfxFamily::Mesh { .. } => "mesh",
            VfxFamily::Trail { .. } => "trail",
            VfxFamily::Decal { .. } => "decal",
            VfxFamily::Light { .. } => "light",
            VfxFamily::PostProcess { .. } => "post-process",
            VfxFamily::Feedback { .. } => "feedback",
        };
        assert_eq!(family, expected);
    }
}

#[test]
fn particle_preserves_ranges_reactions_and_visuals() {
    let effect: VfxEffectDescriptor = serde_json::from_str(PARTICLE).unwrap();
    let VfxFamily::Particle { logical, visual } = effect.family else {
        panic!("expected particle")
    };
    assert_eq!(logical.init.life_time_seconds, [0.6, 1.2]);
    let particle::ParticleVelocity::Polar {
        angle_degrees,
        speed,
    } = logical.init.velocity
    else {
        panic!()
    };
    assert_eq!(angle_degrees, [200.0, 340.0]);
    assert_eq!(speed, [80.0, 160.0]);
    assert_eq!(logical.reaction.len(), 1);
    assert_eq!(logical.evolution.accel, [0.0, 200.0]);
    assert_eq!(logical.evolution.drag, 1.5);
    assert!(logical.logical_death.on_death.disable_collision);
    assert!(matches!(
        logical.init.shape,
        VfxShape::Disk {
            radius: 4.0,
            spawn_location: VfxSpawnLocation::Interior
        }
    ));
    assert!(matches!(
        visual.animations["flicker"],
        particle::ParticleAnimation::Flipbook { looping: true, .. }
    ));
    assert!(matches!(
        visual.visual_disappearance.alpha.from,
        VfxVisualValue::Current
    ));
    assert!(matches!(
        visual.visual_disappearance.size_multiplier.from,
        VfxVisualValue::Value(1.0)
    ));
    assert!(visual.transitions[0].target.color.is_some());
}

#[test]
fn rejects_unknown_tags_and_invalid_values() {
    let original: serde_json::Value = serde_json::from_str(PARTICLE).unwrap();
    for (pointer, replacement) in [
        ("/family", serde_json::json!("unknown")),
        (
            "/visual/representations/0/type",
            serde_json::json!("unknown"),
        ),
        (
            "/visual/visual_disappearance/alpha/from",
            serde_json::json!("unknown"),
        ),
        (
            "/logical/initialization/lifetime_seconds/min",
            serde_json::json!("invalid"),
        ),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            serde_json::from_value::<VfxEffectDescriptor>(value).is_err(),
            "{pointer}"
        );
    }
}
