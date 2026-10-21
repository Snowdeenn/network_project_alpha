//! Données vivantes, lues ensuite par le rendu.
//!
//! Flux : manager.simulate -> update_<famille> -> émission, vieillissement,
//! transitions visuelles, disparition, puis mise à jour de l'état global.
//! Les descriptions restent immuables. Aucun chargement de ressource ici.
//!
//! Mesh/decal/light : l'émission crée des éléments, chacun avec sa durée de vie.
//! Le trail émet des points jusqu'à sa durée d'émission ou sa durée logique.
//! Le post-process est unique et vit pendant la durée indiquée par sa description.
//! Les transitions sont parcourues dans l'ordre de déclaration, en suivant from/to.
//! Pour une source attachée, l'appelant actualise params.pos avant simulate.

use std::collections::HashMap;

use math::Vec2;
use rand::RngExt;

use super::instance::{VfxInstance, VfxInstanceState, VfxSpawnParams, sample_disk};
use super::parser::r#struct::*;

pub enum VfxSimulation {
    Particle, // Les particules vivent déjà dans le ParticlePool du manager.
    Mesh(Vec<SurfaceElement>),
    Decal(Vec<SurfaceElement>),
    Light(Vec<LightElement>),
    Trail(TrailSimulation),
    PostProcess(PostProcessSimulation),
}

/// Mesh et decal partagent les mêmes propriétés de surface.
pub struct SurfaceElement {
    pub pos: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub rotation_degrees: f32,
    offset: Vec2,
}

pub struct LightElement {
    pub pos: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub casts_shadows: bool,
    offset: Vec2,
}

pub struct TrailPoint {
    pub pos: Vec2,
    pub age: f32,
    pub width: f32,
    pub color: [f32; 4],
    offset: Vec2,
}

pub struct TrailSimulation {
    pub points: Vec<TrailPoint>,
    pub lifetime: f32,
    elapsed: f64,
    fractional_count: f64,
    burst_emitted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PostProcessValue {
    Scalar(f32),
    Vector(Vec<f32>),
}

pub struct PostProcessSimulation {
    pub age: f32,
    pub lifetime: f32,
    pub weight: f32,
    pub parameters: HashMap<String, PostProcessValue>,
    finishing_at: Option<f32>,
}

impl VfxSimulation {
    pub(super) fn new(family: &VfxFamily, random: &mut impl RngExt) -> Self {
        match family {
            VfxFamily::Particle { .. } => Self::Particle,
            VfxFamily::Mesh { .. } => Self::Mesh(Vec::new()),
            VfxFamily::Decal { .. } => Self::Decal(Vec::new()),
            VfxFamily::Light { .. } => Self::Light(Vec::new()),
            VfxFamily::Trail { logical, .. } => Self::Trail(TrailSimulation {
                points: Vec::new(),
                lifetime: sample_lifetime(logical.init.life_time_seconds, random),
                elapsed: 0.0,
                fractional_count: 0.0,
                burst_emitted: false,
            }),
            VfxFamily::PostProcess { logical, visual } => {
                Self::PostProcess(PostProcessSimulation {
                    age: 0.0,
                    lifetime: sample_lifetime(logical.init.life_time_seconds, random),
                    weight: visual.appearance.weight,
                    parameters: copy_parameters(&visual.appearance.parameters),
                    finishing_at: None,
                })
            }
        }
    }
}

pub(super) fn update_mesh(
    instance: &mut VfxInstance,
    logical: &MeshLogicalDescriptor,
    visual: &MeshVisualDescriptor,
    dt: f32,
) {
    let ages = emitted_ages(instance, &logical.emission, dt);
    let Some(VfxSimulation::Mesh(elements)) = &mut instance.simulation else {
        unreachable!()
    };
    for element in elements.iter_mut() {
        element.age += dt;
    }
    for age in ages {
        let offset = sample_offset(&logical.init.shape, &mut instance.random);
        elements.push(SurfaceElement {
            pos: placed(offset, &instance.params),
            offset,
            age,
            lifetime: sample_lifetime(logical.init.life_time_seconds, &mut instance.random),
            size: visual.appearance.size,
            color: visual.appearance.color,
            rotation_degrees: visual.appearance.rotation_degrees,
        });
    }

    for element in elements.iter_mut() {
        follow_source(
            &mut element.pos,
            element.offset,
            &logical.init.space,
            &logical.init.shape,
            &instance.params,
        );
        // Recalcul depuis l'apparence initiale : pas d'interpolation cumulative
        // dépendant du nombre de frames. Une transition commence après la précédente.
        element.size = visual.appearance.size;
        element.color = visual.appearance.color;
        element.rotation_degrees = visual.appearance.rotation_degrees;
        let age = element.age.min(element.lifetime);
        let mut state = visual.appearance.initial_state.as_str();
        let mut end = 0.0_f32;
        for transition in &visual.transitions {
            if transition.from != state {
                continue;
            }
            let start = transition_start(&transition.when, element.lifetime).max(end);
            if age < start {
                continue;
            }
            let t = progress(age - start, transition.duration_seconds);
            if let Some(size) = transition.target.size {
                element.size = mix_array(element.size, size, t);
            }
            if let Some(color) = transition.target.color {
                element.color = mix_array(element.color, color, t);
            }
            if let Some(rotation) = transition.target.rotation_degrees {
                element.rotation_degrees = mix(element.rotation_degrees, rotation, t);
            }
            if t < 1.0 {
                break;
            }
            state = &transition.to;
            end = start + transition.duration_seconds;
        }
        let fade = &visual.visual_disappearance;
        if element.age >= element.lifetime {
            let t = progress(element.age - element.lifetime, fade.duration_seconds);
            element.color[3] = fade_value(&fade.alpha, element.color[3], t);
            let scale = fade_value(&fade.size_multiplier, 1.0, t);
            element.size = element.size.map(|value| value * scale);
        }
        element.size = element.size.map(|value| value * instance.params.scale);
    }
    elements.retain(|element| {
        element.age < element.lifetime + visual.visual_disappearance.duration_seconds
    });
    finish_emitter(
        &mut instance.state,
        instance
            .emission
            .finished(&logical.emission, instance.params.lifetime),
        elements.is_empty(),
    );
}

pub(super) fn update_decal(
    instance: &mut VfxInstance,
    logical: &DecalLogicalDescriptor,
    visual: &DecalVisualDescriptor,
    dt: f32,
) {
    let ages = emitted_ages(instance, &logical.emission, dt);
    let Some(VfxSimulation::Decal(elements)) = &mut instance.simulation else {
        unreachable!()
    };
    for element in elements.iter_mut() {
        element.age += dt;
    }
    for age in ages {
        let offset = sample_offset(&logical.init.shape, &mut instance.random);
        elements.push(SurfaceElement {
            pos: placed(offset, &instance.params),
            offset,
            age,
            lifetime: sample_lifetime(logical.init.life_time_seconds, &mut instance.random),
            size: visual.appearance.size,
            color: visual.appearance.color,
            rotation_degrees: visual.appearance.rotation_degrees,
        });
    }
    for element in elements.iter_mut() {
        follow_source(
            &mut element.pos,
            element.offset,
            &logical.init.space,
            &logical.init.shape,
            &instance.params,
        );
        element.size = visual.appearance.size;
        element.color = visual.appearance.color;
        element.rotation_degrees = visual.appearance.rotation_degrees;
        let age = element.age.min(element.lifetime);
        let mut state = visual.appearance.initial_state.as_str();
        let mut end = 0.0_f32;
        for transition in &visual.transitions {
            if transition.from != state {
                continue;
            }
            let start = transition_start(&transition.when, element.lifetime).max(end);
            if age < start {
                continue;
            }
            let t = progress(age - start, transition.duration_seconds);
            if let Some(size) = transition.target.size {
                element.size = mix_array(element.size, size, t);
            }
            if let Some(color) = transition.target.color {
                element.color = mix_array(element.color, color, t);
            }
            if let Some(rotation) = transition.target.rotation_degrees {
                element.rotation_degrees = mix(element.rotation_degrees, rotation, t);
            }
            if t < 1.0 {
                break;
            }
            state = &transition.to;
            end = start + transition.duration_seconds;
        }
        if element.age >= element.lifetime {
            let fade = &visual.visual_disappearance;
            element.color[3] = fade_value(
                &fade.alpha,
                element.color[3],
                progress(element.age - element.lifetime, fade.duration_seconds),
            );
        }
        element.size = element.size.map(|value| value * instance.params.scale);
    }
    elements.retain(|element| {
        element.age < element.lifetime + visual.visual_disappearance.duration_seconds
    });
    finish_emitter(
        &mut instance.state,
        instance
            .emission
            .finished(&logical.emission, instance.params.lifetime),
        elements.is_empty(),
    );
}

pub(super) fn update_light(
    instance: &mut VfxInstance,
    logical: &LightLogicalDescriptor,
    visual: &LightVisualDescriptor,
    dt: f32,
) {
    let ages = emitted_ages(instance, &logical.emission, dt);
    let Some(VfxSimulation::Light(elements)) = &mut instance.simulation else {
        unreachable!()
    };
    for element in elements.iter_mut() {
        element.age += dt;
    }
    for age in ages {
        let offset = sample_offset(&logical.init.shape, &mut instance.random);
        elements.push(LightElement {
            pos: placed(offset, &instance.params),
            offset,
            age,
            lifetime: sample_lifetime(logical.init.life_time_seconds, &mut instance.random),
            color: visual.appearance.color,
            intensity: visual.appearance.intensity,
            radius: visual.appearance.radius,
            casts_shadows: visual.appearance.casts_shadows,
        });
    }
    for element in elements.iter_mut() {
        follow_source(
            &mut element.pos,
            element.offset,
            &logical.init.space,
            &logical.init.shape,
            &instance.params,
        );
        element.color = visual.appearance.color;
        element.intensity = visual.appearance.intensity;
        element.radius = visual.appearance.radius;
        let age = element.age.min(element.lifetime);
        let mut state = visual.appearance.initial_state.as_str();
        let mut end = 0.0_f32;
        for transition in &visual.transitions {
            if transition.from != state {
                continue;
            }
            let start = transition_start(&transition.when, element.lifetime).max(end);
            if age < start {
                continue;
            }
            let t = progress(age - start, transition.duration_seconds);
            if let Some(color) = transition.target.color {
                element.color = mix_array(element.color, color, t);
            }
            if let Some(intensity) = transition.target.intensity {
                element.intensity = mix(element.intensity, intensity, t);
            }
            if let Some(radius) = transition.target.radius {
                element.radius = mix(element.radius, radius, t);
            }
            if t < 1.0 {
                break;
            }
            state = &transition.to;
            end = start + transition.duration_seconds;
        }
        if element.age >= element.lifetime {
            let fade = &visual.visual_disappearance;
            element.intensity = fade_value(
                &fade.intensity,
                element.intensity,
                progress(element.age - element.lifetime, fade.duration_seconds),
            );
        }
        element.radius *= instance.params.scale;
    }
    elements.retain(|element| {
        element.age < element.lifetime + visual.visual_disappearance.duration_seconds
    });
    finish_emitter(
        &mut instance.state,
        instance
            .emission
            .finished(&logical.emission, instance.params.lifetime),
        elements.is_empty(),
    );
}

pub(super) fn update_trail(
    instance: &mut VfxInstance,
    logical: &TrailLogicalDescription,
    visual: &TrailVisualDescriptor,
    dt: f32,
) {
    let Some(VfxSimulation::Trail(trail)) = &mut instance.simulation else {
        unreachable!()
    };
    let previous = trail.elapsed;
    trail.elapsed += f64::from(dt);
    for point in &mut trail.points {
        point.age += dt;
    }

    let emission = &logical.emission;
    let delay = f64::from(emission.delay_seconds);
    // La durée logique du trail est comptée depuis son activation.
    let end = (delay + f64::from(emission.duration_seconds)).min(f64::from(trail.lifetime));
    let rate = f64::from(emission.rate_per_second);
    assert!(rate.is_finite() && rate >= 0.0);
    let mut count = 0;
    let mut last_emitted_at = delay;
    if instance.state == VfxInstanceState::Active && trail.elapsed >= delay && end >= delay {
        match emission.mode {
            VfxEmissionMode::Burst => {
                // Le format trail n'a pas de count : un burst crée un point.
                if !trail.burst_emitted {
                    count = 1;
                    trail.burst_emitted = true;
                }
            }
            VfxEmissionMode::Continuous => {
                let emitting_dt = (trail.elapsed.min(end) - previous.max(delay)).max(0.0);
                trail.fractional_count += emitting_dt * rate;
                count = trail.fractional_count.floor() as usize;
                trail.fractional_count -= count as f64;
                if rate > 0.0 {
                    last_emitted_at = trail.elapsed.min(end) - trail.fractional_count / rate;
                }
            }
        }
    }
    let max_points = logical.evolution.max_points as usize;
    // Seuls les points les plus récents peuvent survivre à la limite du trail.
    for index in count.saturating_sub(max_points)..count {
        let born_at = if matches!(emission.mode, VfxEmissionMode::Continuous) {
            last_emitted_at - (count - 1 - index) as f64 / rate
        } else {
            delay
        };
        let offset = sample_offset(&logical.init.shape, &mut instance.random);
        trail.points.push(TrailPoint {
            pos: placed(offset, &instance.params),
            offset,
            age: (trail.elapsed - born_at).max(0.0) as f32,
            width: visual.appearance.width,
            color: visual.appearance.color,
        });
    }
    if trail.points.len() > max_points {
        trail.points.drain(..trail.points.len() - max_points);
    }

    let mut width = visual.appearance.width;
    let mut color = visual.appearance.color;
    let age = (trail.elapsed as f32).min(trail.lifetime);
    let mut state = visual.appearance.initial_state.as_str();
    let mut transition_end = 0.0_f32;
    for transition in &visual.transitions {
        if transition.from != state {
            continue;
        }
        let start = transition_start(&transition.when, trail.lifetime).max(transition_end);
        if age < start {
            continue;
        }
        let t = progress(age - start, transition.duration_seconds);
        if let Some(target) = transition.target.width {
            width = mix(width, target, t);
        }
        if let Some(target) = transition.target.color {
            color = mix_array(color, target, t);
        }
        if t < 1.0 {
            break;
        }
        state = &transition.to;
        transition_end = start + transition.duration_seconds;
    }
    for point in &mut trail.points {
        // En espace monde, les anciens points restent en place : seule la
        // nouvelle extrémité suit params.pos, même pour une source attachée.
        if matches!(logical.init.space, VfxSpace::Local) {
            point.pos = placed(point.offset, &instance.params);
        }
        point.width = width * instance.params.scale;
        point.color = color;
        if point.age >= logical.evolution.point_lifetime_seconds {
            let fade = &visual.visual_disappearance;
            point.color[3] = fade_value(
                &fade.alpha,
                color[3],
                progress(
                    point.age - logical.evolution.point_lifetime_seconds,
                    fade.duration_seconds,
                ),
            );
        }
    }
    trail.points.retain(|point| {
        point.age
            < logical.evolution.point_lifetime_seconds
                + visual.visual_disappearance.duration_seconds
    });
    let emission_done = trail.elapsed >= end || trail.burst_emitted;
    finish_emitter(&mut instance.state, emission_done, trail.points.is_empty());
}

pub(super) fn update_post_process(
    instance: &mut VfxInstance,
    visual: &PostProcessVisualDescriptor,
    dt: f32,
) {
    let Some(VfxSimulation::PostProcess(effect)) = &mut instance.simulation else {
        unreachable!()
    };
    // Un arrêt explicite commence le fondu à l'âge précédant cette frame.
    if instance.state == VfxInstanceState::Finishing && effect.finishing_at.is_none() {
        effect.finishing_at = Some(effect.age.min(effect.lifetime));
    }
    effect.age += dt;
    if effect.age >= effect.lifetime && effect.finishing_at.is_none() {
        effect.finishing_at = Some(effect.lifetime);
    }
    let age = effect
        .age
        .min(effect.finishing_at.unwrap_or(effect.lifetime));
    effect.weight = visual.appearance.weight;
    effect.parameters = copy_parameters(&visual.appearance.parameters);
    let mut state = visual.appearance.initial_state.as_str();
    let mut end = 0.0_f32;
    for transition in &visual.transitions {
        if transition.from != state {
            continue;
        }
        let start = transition_start(&transition.when, effect.lifetime).max(end);
        if age < start {
            continue;
        }
        let t = progress(age - start, transition.duration_seconds);
        if let Some(weight) = transition.target.weight {
            effect.weight = mix(effect.weight, weight, t);
        }
        if let Some(parameters) = &transition.target.parameters {
            for (name, target) in parameters {
                let target = copy_parameter(target);
                match (effect.parameters.get_mut(name), target) {
                    (Some(PostProcessValue::Scalar(value)), PostProcessValue::Scalar(target)) => {
                        *value = mix(*value, target, t)
                    }
                    (Some(PostProcessValue::Vector(value)), PostProcessValue::Vector(target))
                        if value.len() == target.len() =>
                    {
                        for (value, target) in value.iter_mut().zip(target) {
                            *value = mix(*value, target, t);
                        }
                    }
                    // Un nouveau paramètre ou changement de type prend effet en fin de transition.
                    (_, target) if t >= 1.0 => {
                        effect.parameters.insert(name.clone(), target);
                    }
                    _ => {}
                }
            }
        }
        if t < 1.0 {
            break;
        }
        state = &transition.to;
        end = start + transition.duration_seconds;
    }
    if let Some(start) = effect.finishing_at {
        let fade = &visual.visual_disappearance;
        let t = progress(effect.age - start, fade.duration_seconds);
        effect.weight = fade_value(&fade.weight, effect.weight, t);
        instance.state = if t >= 1.0 {
            VfxInstanceState::Finished
        } else {
            VfxInstanceState::Finishing
        };
    }
}

// Petits calculs communs, sans scheduler ni système de rendu supplémentaire.

/// Âge des nouveaux éléments en fin de frame. Un délai traversé par une grosse
/// frame ne décale pas leur naissance jusqu'à la frame suivante.
fn emitted_ages(instance: &mut VfxInstance, emission: &VfxEmission, dt: f32) -> Vec<f32> {
    if instance.state != VfxInstanceState::Active {
        return Vec::new();
    }
    let count = instance
        .emission
        .advance(emission, dt, instance.params.lifetime);
    let now = instance.emission.elapsed;
    let delay = f64::from(emission.delay_seconds);
    match emission.mode {
        VfxEmissionMode::Burst => vec![(now - delay).max(0.0) as f32; count as usize],
        VfxEmissionMode::Continuous => {
            if count == 0 {
                return Vec::new();
            }
            let rate = f64::from(emission.count);
            let end = now.min(delay + f64::from(instance.params.lifetime));
            let last = end - instance.emission.fractional_count / rate;
            (0..count)
                .map(|index| (now - (last - f64::from(count - 1 - index) / rate)).max(0.0) as f32)
                .collect()
        }
    }
}

fn finish_emitter(state: &mut VfxInstanceState, emission_done: bool, empty: bool) {
    if emission_done || *state == VfxInstanceState::Finishing {
        *state = if empty {
            VfxInstanceState::Finished
        } else {
            VfxInstanceState::Finishing
        };
    }
}

fn sample_lifetime(range: [f32; 2], random: &mut impl RngExt) -> f32 {
    random.random_range(range[0]..=range[1])
}

fn sample_offset(shape: &VfxShape, random: &mut impl RngExt) -> Vec2 {
    match shape {
        VfxShape::Point { coord } => Vec2::new(coord[0], coord[1]),
        VfxShape::Disk {
            radius,
            spawn_location,
        } => sample_disk(random, *radius, spawn_location),
        // L'appelant fournit la position résolue de la source via params.pos.
        VfxShape::Attachement { .. } => Vec2::zero(),
    }
}

fn placed(offset: Vec2, params: &VfxSpawnParams) -> Vec2 {
    params.pos + offset * params.scale
}

fn follow_source(
    pos: &mut Vec2,
    offset: Vec2,
    space: &VfxSpace,
    shape: &VfxShape,
    params: &VfxSpawnParams,
) {
    if matches!(space, VfxSpace::Local) || matches!(shape, VfxShape::Attachement { .. }) {
        *pos = placed(offset, params);
    }
}

fn transition_start(event: &VfxTransitionEvent, lifetime: f32) -> f32 {
    match event {
        VfxTransitionEvent::NormalizedAge { at_least } => at_least * lifetime,
    }
}

fn progress(elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        1.0
    } else {
        (elapsed / duration).clamp(0.0, 1.0)
    }
}

fn mix(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn mix_array<const N: usize>(from: [f32; N], to: [f32; N], t: f32) -> [f32; N] {
    std::array::from_fn(|index| mix(from[index], to[index], t))
}

fn fade_value(range: &VfxVisualRange, current: f32, t: f32) -> f32 {
    let from = match range.from {
        VfxVisualValue::Current => current,
        VfxVisualValue::Value(value) => value,
    };
    mix(from, range.to, t)
}

fn copy_parameter(value: &PostProcessParameter) -> PostProcessValue {
    match value {
        PostProcessParameter::Scalar(value) => PostProcessValue::Scalar(*value),
        PostProcessParameter::Vector(value) => PostProcessValue::Vector(value.clone()),
    }
}

fn copy_parameters(
    parameters: &HashMap<String, PostProcessParameter>,
) -> HashMap<String, PostProcessValue> {
    parameters
        .iter()
        .map(|(name, value)| (name.clone(), copy_parameter(value)))
        .collect()
}

#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
