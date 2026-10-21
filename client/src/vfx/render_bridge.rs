//! Extraction possédée, puis adaptation aux capacités actuelles de Prism.
//! Les textures et matériaux des exemples ne sont pas fournis dans le dépôt :
//! les surfaces et rubans ont un fallback géométrique ; lumière/post-process
//! restent des intentions inspectables, sans créer de pipelines dans le runtime.

use super::VfxSystem;
use super::parser::r#struct::{VfxBlend, VfxFamily};
use super::simulation::{PostProcessValue, VfxSimulation};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VfxSprite {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub rotation_degrees: f32,
    pub color: [f32; 4],
    pub additive: bool,
}

pub struct VfxRibbon {
    pub points: Vec<([f32; 2], f32, [f32; 4])>,
    pub additive: bool,
}

pub struct VfxLight {
    pub pos: [f32; 2],
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
}

pub struct VfxPostProcess {
    pub effect: String,
    pub weight: f32,
    pub parameters: HashMap<String, PostProcessValue>,
}

#[derive(Default)]
pub struct VfxRenderSnapshot {
    pub sprites: Vec<VfxSprite>,
    pub ribbons: Vec<VfxRibbon>,
    pub lights: Vec<VfxLight>,
    pub post_process: Vec<VfxPostProcess>,
}

impl VfxSystem {
    pub fn extract(&self) -> VfxRenderSnapshot {
        let mut snapshot = VfxRenderSnapshot::default();
        for (owner, particle) in self.instances.particles() {
            let instance = self.instances.get(owner).unwrap();
            let effect = self.registry.assets.get(instance.descriptor).unwrap();
            let VfxFamily::Particle { visual, .. } = &effect.family else {
                continue;
            };
            let mut color = visual.appearance.color;
            let tint = [
                particle.color.r,
                particle.color.g,
                particle.color.b,
                particle.color.a,
            ]
            .map(|v| v as f32 / 255.0);
            let mut size = visual.appearance.size;
            let age = (particle.lt_max - particle.lifetime).min(particle.lt_max);
            let mut state = visual.appearance.initial_state.as_str();
            let mut end = 0.0_f32;
            for transition in &visual.transitions {
                if transition.from != state {
                    continue;
                }
                let super::parser::r#struct::VfxTransitionEvent::NormalizedAge { at_least } =
                    transition.when;
                let start = (at_least * particle.lt_max).max(end);
                if age < start {
                    continue;
                }
                let t = progress(age - start, transition.duration_seconds);
                if let Some(target) = transition.target.color {
                    color = mix(color, target, t);
                }
                if let Some(target) = transition.target.size {
                    size = mix(size, target, t);
                }
                if t < 1.0 {
                    break;
                }
                state = &transition.to;
                end = start + transition.duration_seconds;
            }
            let mut scale = particle.scale;
            if particle.lifetime <= 0.0 {
                let fade = &visual.visual_disappearance;
                let t = progress(-particle.lifetime, fade.duration_seconds);
                color[3] = fade_value(&fade.alpha, color[3], t);
                scale *= fade_value(&fade.size_multiplier, 1.0, t);
            }
            snapshot.sprites.push(VfxSprite {
                pos: [particle.pos.x, particle.pos.y],
                size: size.map(|v| v * scale),
                rotation_degrees: if matches!(
                    visual.appearance.orientation,
                    super::parser::r#struct::VfxOrientation::Velocity
                ) {
                    particle.velocity.y.atan2(particle.velocity.x).to_degrees()
                } else {
                    0.0
                },
                color: std::array::from_fn(|i| color[i] * tint[i]),
                additive: matches!(visual.appearance.blend, VfxBlend::Additive),
            });
        }
        for (_, components) in self.components() {
            for &id in components {
                let instance = self.instances.get(id).unwrap();
                let effect = self.registry.assets.get(instance.descriptor).unwrap();
                match instance.simulation() {
                    Some(VfxSimulation::Mesh(elements)) | Some(VfxSimulation::Decal(elements)) => {
                        let additive = match &effect.family {
                            VfxFamily::Mesh { visual, .. } => {
                                matches!(visual.appearance.blend, VfxBlend::Additive)
                            }
                            VfxFamily::Decal { visual, .. } => {
                                matches!(visual.appearance.blend, VfxBlend::Additive)
                            }
                            _ => false,
                        };
                        for element in elements {
                            snapshot.sprites.push(VfxSprite {
                                pos: [element.pos.x, element.pos.y],
                                size: element.size,
                                rotation_degrees: element.rotation_degrees,
                                color: element.color,
                                additive,
                            });
                        }
                    }
                    Some(VfxSimulation::Trail(trail)) => {
                        let additive = matches!(&effect.family, VfxFamily::Trail { visual, .. } if matches!(visual.appearance.blend, VfxBlend::Additive));
                        snapshot.ribbons.push(VfxRibbon {
                            points: trail
                                .points
                                .iter()
                                .map(|p| ([p.pos.x, p.pos.y], p.width, p.color))
                                .collect(),
                            additive,
                        });
                    }
                    Some(VfxSimulation::Light(elements)) => {
                        for element in elements {
                            snapshot.lights.push(VfxLight {
                                pos: [element.pos.x, element.pos.y],
                                color: element.color,
                                intensity: element.intensity,
                                radius: element.radius,
                            });
                        }
                    }
                    Some(VfxSimulation::PostProcess(effect_state)) => {
                        snapshot.post_process.push(VfxPostProcess {
                            effect: effect.name.clone(),
                            weight: effect_state.weight,
                            parameters: effect_state.parameters.clone(),
                        })
                    }
                    _ => {}
                }
            }
        }
        snapshot
    }
}

pub fn append_to_frame(snapshot: &VfxRenderSnapshot, frame: &mut prism::Frame) {
    for sprite in &snapshot.sprites {
        frame.push_vfx(prism::DrawCommand::Shape {
            shape: prism::Shape::Quad {
                pos: [
                    sprite.pos[0] - sprite.size[0] * 0.5,
                    sprite.pos[1] - sprite.size[1] * 0.5,
                ],
                size: sprite.size,
                rotation: sprite.rotation_degrees.to_radians(),
                color: sprite.color,
                uv: None,
            },
            blend: blend(sprite.additive),
            layer: 0,
        });
    }
    for ribbon in &snapshot.ribbons {
        for pair in ribbon.points.windows(2) {
            frame.push_vfx(prism::DrawCommand::Shape {
                shape: prism::Shape::Line {
                    start: pair[0].0,
                    end: pair[1].0,
                    thickness: pair[0].1.min(pair[1].1),
                    color: pair[1].2,
                },
                blend: blend(ribbon.additive),
                layer: 1,
            });
        }
    }
}

fn blend(additive: bool) -> prism::BlendMode {
    if additive {
        prism::BlendMode::Additive
    } else {
        prism::BlendMode::Alpha
    }
}

fn progress(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        1.0
    } else {
        (age / duration).clamp(0.0, 1.0)
    }
}

fn mix<const N: usize>(from: [f32; N], to: [f32; N], t: f32) -> [f32; N] {
    std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t)
}

fn fade_value(range: &super::parser::r#struct::VfxVisualRange, current: f32, t: f32) -> f32 {
    use super::parser::r#struct::VfxVisualValue;
    let from = match range.from {
        VfxVisualValue::Current => current,
        VfxVisualValue::Value(value) => value,
    };
    from + (range.to - from) * t
}
