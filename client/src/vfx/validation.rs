use serde_json::Value;
use std::io;

use super::parser::r#struct::*;
use super::registry::invalid;

/// Validation numérique avant désérialisation en f32 : refuse aussi les nombres
/// JSON finis en f64 qui déborderaient dans la simulation.
pub(super) fn validate_json(source: &str) -> io::Result<()> {
    fn visit(value: &Value, path: &str) -> io::Result<()> {
        match value {
            Value::Number(number) => {
                if !number
                    .as_f64()
                    .is_some_and(|value| (value as f32).is_finite())
                {
                    return Err(invalid(format!("{path}: expected a finite f32")));
                }
            }
            Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    visit(value, &format!("{path}[{index}]"))?;
                }
            }
            Value::Object(fields) => {
                if let (Some(min), Some(max)) = (fields.get("min"), fields.get("max")) {
                    if min
                        .as_f64()
                        .zip(max.as_f64())
                        .is_some_and(|(min, max)| min > max)
                    {
                        return Err(invalid(format!("{path}: min exceeds max")));
                    }
                    if min
                        .as_array()
                        .zip(max.as_array())
                        .is_some_and(|(min, max)| {
                            min.iter().zip(max).any(|(min, max)| {
                                min.as_f64()
                                    .zip(max.as_f64())
                                    .is_some_and(|(min, max)| min > max)
                            })
                        })
                    {
                        return Err(invalid(format!("{path}: box min exceeds max")));
                    }
                }
                for (name, value) in fields {
                    let field_path = format!("{path}.{name}");
                    visit(value, &field_path)?;
                    let limited = name.ends_with("_seconds")
                        || matches!(
                            name.as_str(),
                            "count"
                                | "max_points"
                                | "rate_per_second"
                                | "radius"
                                | "width"
                                | "intensity"
                                | "linear_drag"
                                | "size"
                                | "size_multiplier"
                                | "speed"
                                | "fps"
                                | "weight"
                                | "alpha"
                                | "at_least"
                                | "color"
                                | "scale"
                                | "trauma"
                        );
                    if limited {
                        fn check(value: &Value, path: &str, max: f64) -> io::Result<()> {
                            match value {
                                Value::Number(number)
                                    if !number
                                        .as_f64()
                                        .is_some_and(|v| (0.0..=max).contains(&v)) =>
                                {
                                    return Err(invalid(format!(
                                        "{path}: value outside 0..={max}"
                                    )));
                                }
                                Value::Array(values) => {
                                    for value in values {
                                        check(value, path, max)?;
                                    }
                                }
                                Value::Object(fields) => {
                                    for value in fields.values() {
                                        check(value, path, max)?;
                                    }
                                }
                                _ => {}
                            }
                            Ok(())
                        }
                        check(
                            value,
                            &field_path,
                            if matches!(
                                name.as_str(),
                                "weight" | "alpha" | "at_least" | "color" | "trauma"
                            ) {
                                1.0
                            } else if name.ends_with("_seconds") {
                                60.0
                            } else {
                                4096.0
                            },
                        )?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(&serde_json::from_str::<Value>(source)?, "effect")
}

pub(super) fn validate_descriptor(effect: &VfxEffectDescriptor) -> io::Result<()> {
    if effect.version != 1 || effect.name.trim().is_empty() {
        return Err(invalid("effect: expected version 1 and a non-empty name"));
    }
    // Les références visuelles et états sont propres à chaque famille. La même
    // vérification s'applique sans changer leurs structures existantes.
    fn visual<T>(
        representation: &str,
        representations: Vec<&str>,
        initial: &str,
        states: &std::collections::HashMap<String, T>,
        transitions: Vec<(&str, &str)>,
    ) -> io::Result<()> {
        if !representations.contains(&representation) {
            return Err(invalid(
                "visual.appearance.representation: unknown reference",
            ));
        }
        let unique: std::collections::HashSet<_> = representations.iter().collect();
        if unique.len() != representations.len()
            || representations.iter().any(|name| name.is_empty())
        {
            return Err(invalid("visual.representations: duplicate or empty name"));
        }
        if !states.contains_key(initial) {
            return Err(invalid("visual.appearance.initial_state: unknown state"));
        }
        for (from, to) in transitions {
            if !states.contains_key(from) || !states.contains_key(to) {
                return Err(invalid("visual.transitions: unknown state"));
            }
        }
        Ok(())
    }
    match &effect.family {
        VfxFamily::Feedback { .. } => Ok(()),
        VfxFamily::Particle { visual: v, .. } => {
            for state in v.states.values() {
                if let Some(name) = &state.animation {
                    if !v.animations.contains_key(name) {
                        return Err(invalid("visual.states.animation: unknown animation"));
                    }
                }
            }
            visual(
                &v.appearance.representation,
                v.representations.iter().map(|r| r.name.as_str()).collect(),
                &v.appearance.initial_state,
                &v.states,
                v.transitions
                    .iter()
                    .map(|t| (t.from.as_str(), t.to.as_str()))
                    .collect(),
            )
        }
        VfxFamily::Mesh { visual: v, .. } => {
            for representation in &v.representations {
                if let MeshRepresentationType::Ring {
                    half_arc_degrees,
                    resolution,
                } = representation.representation_type
                    && (!(0.0..=180.0).contains(&half_arc_degrees)
                        || !(1..=1024).contains(&resolution))
                {
                    return Err(invalid(
                        "ring: expected a half arc in 0..=180 and 1..=1024 segments",
                    ));
                }
            }
            visual(
                &v.appearance.representation,
                v.representations.iter().map(|r| r.name.as_str()).collect(),
                &v.appearance.initial_state,
                &v.states,
                v.transitions
                    .iter()
                    .map(|t| (t.from.as_str(), t.to.as_str()))
                    .collect(),
            )
        }
        VfxFamily::Decal { visual: v, .. } => visual(
            &v.appearance.representation,
            v.representations.iter().map(|r| r.name.as_str()).collect(),
            &v.appearance.initial_state,
            &v.states,
            v.transitions
                .iter()
                .map(|t| (t.from.as_str(), t.to.as_str()))
                .collect(),
        ),
        VfxFamily::Trail { visual: v, .. } => visual(
            &v.appearance.representation,
            v.representations.iter().map(|r| r.name.as_str()).collect(),
            &v.appearance.initial_state,
            &v.states,
            v.transitions
                .iter()
                .map(|t| (t.from.as_str(), t.to.as_str()))
                .collect(),
        ),
        VfxFamily::Light { visual: v, .. } => visual(
            &v.appearance.representation,
            v.representations.iter().map(|r| r.name.as_str()).collect(),
            &v.appearance.initial_state,
            &v.states,
            v.transitions
                .iter()
                .map(|t| (t.from.as_str(), t.to.as_str()))
                .collect(),
        ),
        VfxFamily::PostProcess { visual: v, .. } => visual(
            &v.appearance.representation,
            v.representations.iter().map(|r| r.name.as_str()).collect(),
            &v.appearance.initial_state,
            &v.states,
            v.transitions
                .iter()
                .map(|t| (t.from.as_str(), t.to.as_str()))
                .collect(),
        ),
    }
}
