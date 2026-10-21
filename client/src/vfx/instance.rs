use crate::vfx::parser::r#struct::{
    ParticleLogicalDescriptor, ParticleVisualDescriptor, VfxEmission, VfxEmissionMode, VfxShape,
    VfxSpawnLocation,
};

use super::parser::r#struct::VfxFamily;
use super::storage::AssetRegister;
use math::Vec2;
use rand::RngExt;
use rand::{SeedableRng, rngs::StdRng};

use super::dag::VfxGraph;
use super::parser::r#struct::VfxEffectDescriptor;
use super::particle::*;
use super::simulation::{self, VfxSimulation};
use prism::ids::MaterialId;
use utils::{Arena, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfxInstanceState {
    Waiting,
    Active,
    Finishing,
    Finished,
}

#[derive(Clone, Copy)]
pub struct VfxSpawnParams {
    pub pos: Vec2,
    pub scale: f32,
    pub direction: Vec2,
    /// Durée d'émission continue des particules, meshes, decals et lumières.
    /// Les durées de vie des éléments viennent de leur description.
    pub lifetime: f32,
}

pub struct VfxInstance {
    pub descriptor: Id<VfxEffectDescriptor>,
    pub params: VfxSpawnParams,
    pub age: f32,
    pub state: VfxInstanceState,
    // A voir si il faut un autre type d'id pour ref les éléments de l'effet
    pub materials: Vec<MaterialId>,
    pub(super) simulation: Option<VfxSimulation>,
    pub(super) emission: EmissionState,
    pub(super) random: StdRng,
}

impl VfxInstance {
    /// État calculé pour le rendu, disponible après la première simulation active.
    pub fn simulation(&self) -> Option<&VfxSimulation> {
        self.simulation.as_ref()
    }
}

/// Horloge propre à l'occurrence : le temps d'attente avant activation ne compte pas.
#[derive(Default)]
pub(super) struct EmissionState {
    pub(super) elapsed: f64,
    burst_emitted: bool,
    pub(super) fractional_count: f64,
}

impl EmissionState {
    pub(super) fn advance(&mut self, emission: &VfxEmission, dt: f32, duration: f32) -> u32 {
        assert!(dt.is_finite() && dt >= 0.0);
        assert!(duration.is_finite() && duration >= 0.0);
        assert!(emission.delay_seconds.is_finite() && emission.delay_seconds >= 0.0);
        let previous = self.elapsed;
        self.elapsed += f64::from(dt);
        let delay = f64::from(emission.delay_seconds);
        if self.elapsed < delay {
            return 0;
        }
        match emission.mode {
            VfxEmissionMode::Burst => {
                if self.burst_emitted {
                    return 0;
                }
                self.burst_emitted = true;
                emission.count
            }
            VfxEmissionMode::Continuous => {
                // Seule la fraction de frame située après le délai contribue.
                let end = delay + f64::from(duration);
                let emitting_dt = (self.elapsed.min(end) - previous.max(delay)).max(0.0);
                self.fractional_count += emitting_dt * f64::from(emission.count);
                let count = self.fractional_count.floor() as u32;
                self.fractional_count -= f64::from(count);
                count
            }
        }
    }

    pub(super) fn finished(&self, emission: &VfxEmission, duration: f32) -> bool {
        match emission.mode {
            VfxEmissionMode::Burst => self.burst_emitted,
            VfxEmissionMode::Continuous => {
                self.elapsed >= f64::from(emission.delay_seconds) + f64::from(duration)
            }
        }
    }
}

pub type VfxInstanceId = Id<VfxInstance>;

pub struct VfxInstanceManager {
    instances: Arena<VfxInstance>,
    particles: ParticlePool,
}

impl VfxInstanceManager {
    pub fn get(&self, id: VfxInstanceId) -> Option<&VfxInstance> {
        self.instances.get(id)
    }

    pub fn get_mut(&mut self, id: VfxInstanceId) -> Option<&mut VfxInstance> {
        self.instances.get_mut(id)
    }

    pub fn new() -> Self {
        VfxInstanceManager {
            instances: Arena::new(),
            particles: ParticlePool::new(),
        }
    }

    pub fn create_instance(
        &mut self,
        descriptor: Id<VfxEffectDescriptor>,
        params: VfxSpawnParams,
        materials: Vec<MaterialId>,
    ) -> VfxInstanceId {
        self.instances.insert(VfxInstance {
            descriptor,
            params,
            age: 0.0,
            state: VfxInstanceState::Waiting,
            materials,
            simulation: None,
            emission: EmissionState::default(),
            random: StdRng::seed_from_u64(rand::random()),
        })
    }

    pub fn destroy_instance(&mut self, instance: VfxInstanceId) {
        if self.instances.get(instance).is_none() {
            return;
        }
        self.particles.remove_owner(instance);
        self.instances.remove(instance);
    }

    pub(super) fn set_seed(&mut self, instance: VfxInstanceId, seed: u64) {
        self.instances.get_mut(instance).unwrap().random = StdRng::seed_from_u64(seed);
    }

    pub(super) fn particles(&self) -> impl Iterator<Item = (VfxInstanceId, &Particle)> {
        self.particles.iter()
    }

    /// Une fois par frame : avancer les instances déjà activées par les graphes.
    /// Pour un attachement mesh/decal/light/trail, mettre params.pos à jour avant
    /// cet appel avec la position résolue de la source.
    pub fn simulate(&mut self, dt: f32, descriptions: &AssetRegister) {
        assert!(dt.is_finite() && dt >= 0.0);
        self.particles.update(dt);
        let ids: Vec<_> = self.instances.iter_with_ids().map(|(id, _)| id).collect();
        for id in ids {
            let instance = self.instances.get_mut(id).unwrap();
            if matches!(
                instance.state,
                VfxInstanceState::Waiting | VfxInstanceState::Finished
            ) {
                continue;
            }
            instance.age += dt;
            let description = descriptions.get(instance.descriptor).unwrap();
            // Initialiser une seule fois, à l'activation : create_instance n'a
            // besoin que de l'ID, la description est disponible ici.
            if instance.simulation.is_none() {
                instance.simulation = Some(VfxSimulation::new(
                    &description.family,
                    &mut instance.random,
                ));
            }

            match &description.family {
                VfxFamily::Particle { logical, visual } => {
                    update_particles(id, instance, &mut self.particles, logical, visual, dt)
                }
                VfxFamily::Mesh { logical, visual } => {
                    simulation::update_mesh(instance, logical, visual, dt)
                }
                VfxFamily::Trail { logical, visual } => {
                    simulation::update_trail(instance, logical, visual, dt)
                }
                VfxFamily::Decal { logical, visual } => {
                    simulation::update_decal(instance, logical, visual, dt)
                }
                VfxFamily::Light { logical, visual } => {
                    simulation::update_light(instance, logical, visual, dt)
                }
                VfxFamily::PostProcess { visual, .. } => {
                    simulation::update_post_process(instance, visual, dt)
                }
            }
        }
    }

    pub fn run(&mut self, graph: &mut VfxGraph, dt: f32) {
        graph.update(dt, &mut self.instances);
    }
}

fn update_particles(
    id: VfxInstanceId,
    instance: &mut VfxInstance,
    particles: &mut ParticlePool,
    logical: &ParticleLogicalDescriptor,
    visual: &ParticleVisualDescriptor,
    dt: f32,
) {
    if instance.state == VfxInstanceState::Finishing {
        if particles.live_count(id) == 0 {
            instance.state = VfxInstanceState::Finished;
        }
        return;
    }
    let count = instance
        .emission
        .advance(&logical.emission, dt, instance.params.lifetime);
    let rng = &mut instance.random;
    for _ in 0..count {
        let offset = match &logical.init.shape {
            VfxShape::Disk {
                radius,
                spawn_location,
            } => sample_disk(rng, *radius, spawn_location),
            VfxShape::Point { coord } => Vec2::new(coord[0], coord[1]),
            // La façade résout l'attachement dans params.pos avant simulation.
            VfxShape::Attachement { .. } => Vec2::zero(),
        };
        let spawn_pos = instance.params.pos + offset * instance.params.scale;
        let velocity = sample_velocity(
            rng,
            logical.init.velocity.angle_degrees,
            logical.init.velocity.speed,
        );
        let lifetime =
            rng.random_range(logical.init.life_time_seconds[0]..=logical.init.life_time_seconds[1]);
        particles.spawn(
            id,
            Particle {
                pos: spawn_pos,
                velocity,
                acceleration: Vec2::new(logical.evolution.accel[0], logical.evolution.accel[1]),
                friction: logical.evolution.drag,
                lifetime,
                lt_max: lifetime,
                disappearance_seconds: visual.visual_disappearance.duration_seconds,
                scale: instance.params.scale,
                growth: 0.0,
                color: prism::Color::WHITE,
            },
        );
    }
    if instance
        .emission
        .finished(&logical.emission, instance.params.lifetime)
    {
        instance.state = if particles.live_count(id) == 0 {
            VfxInstanceState::Finished
        } else {
            VfxInstanceState::Finishing
        };
    }
}

pub(super) fn sample_disk(rng: &mut impl RngExt, radius: f32, location: &VfxSpawnLocation) -> Vec2 {
    let angle = rng.random_range(0.0..std::f32::consts::TAU);
    let distance = match location {
        VfxSpawnLocation::Interior => radius * rng.random::<f32>().sqrt(),
        VfxSpawnLocation::Boundary => radius,
    };
    Vec2::new(angle.cos() * distance, angle.sin() * distance)
}

fn sample_velocity(rng: &mut impl RngExt, angles: [f32; 2], speeds: [f32; 2]) -> Vec2 {
    let angle = rng.random_range(angles[0]..=angles[1]).to_radians();
    let speed = rng.random_range(speeds[0]..=speeds[1]);
    Vec2::new(angle.cos() * speed, angle.sin() * speed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn particle_effect(
        mode: VfxEmissionMode,
        count: u32,
    ) -> (AssetRegister, VfxInstanceManager, VfxInstanceId) {
        let mut description = crate::vfx::parser::parse_effect(include_str!(
            "../../../assets/config/vfx/exemples_vfx/particle.json"
        ))
        .unwrap();
        if let VfxFamily::Particle { logical, visual } = &mut description.family {
            logical.emission = VfxEmission {
                mode,
                count,
                delay_seconds: 0.5,
            };
            logical.init.life_time_seconds = [0.5, 0.5];
            visual.visual_disappearance.duration_seconds = 0.25;
        }
        let mut descriptions = AssetRegister::new();
        let descriptor = descriptions.insert(description);
        let mut manager = VfxInstanceManager::new();
        let id = manager.create_instance(
            descriptor,
            VfxSpawnParams {
                pos: Vec2::zero(),
                scale: 1.0,
                direction: Vec2::new(1.0, 0.0),
                lifetime: 1.0,
            },
            Vec::new(),
        );
        manager.get_mut(id).unwrap().state = VfxInstanceState::Active;
        (descriptions, manager, id)
    }

    #[test]
    fn burst_finishes_only_after_its_particles_disappear() {
        let (descriptions, mut manager, id) = particle_effect(VfxEmissionMode::Burst, 2);
        manager.simulate(0.25, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Active);
        manager.simulate(0.25, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finishing);
        assert_eq!(manager.particles.live_count(id), 2);
        manager.simulate(0.5, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finishing);
        manager.simulate(0.25, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
        assert_eq!(manager.particles.live_count(id), 0);
    }

    #[test]
    fn continuous_caps_emission_at_duration_and_handles_zero_duration() {
        let (descriptions, mut manager, id) = particle_effect(VfxEmissionMode::Continuous, 4);
        manager.simulate(10.0, &descriptions);
        assert_eq!(manager.particles.live_count(id), 4);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finishing);
        manager.simulate(1.0, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);

        let (descriptions, mut manager, id) = particle_effect(VfxEmissionMode::Continuous, 4);
        manager.get_mut(id).unwrap().params.lifetime = 0.0;
        manager.simulate(0.5, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
        assert_eq!(manager.particles.live_count(id), 0);
    }

    #[test]
    fn finishing_is_per_instance_and_destroy_releases_owned_particles() {
        let (descriptions, mut manager, first) = particle_effect(VfxEmissionMode::Burst, 2);
        let second = manager.create_instance(
            manager.get(first).unwrap().descriptor,
            VfxSpawnParams {
                pos: Vec2::zero(),
                scale: 1.0,
                direction: Vec2::new(1.0, 0.0),
                lifetime: 1.0,
            },
            Vec::new(),
        );
        manager.simulate(0.5, &descriptions);
        manager.get_mut(second).unwrap().state = VfxInstanceState::Active;
        manager.simulate(0.5, &descriptions);
        manager.simulate(0.25, &descriptions);
        assert_eq!(
            manager.get(first).unwrap().state,
            VfxInstanceState::Finished
        );
        assert_eq!(
            manager.get(second).unwrap().state,
            VfxInstanceState::Finishing
        );
        assert_eq!(manager.particles.live_count(second), 2);
        manager.destroy_instance(second);
        assert_eq!(manager.particles.live_count(second), 0);
    }

    #[test]
    fn full_pool_does_not_keep_an_unemitted_effect_finishing() {
        let (descriptions, mut manager, id) = particle_effect(VfxEmissionMode::Burst, 2048);
        manager.simulate(0.5, &descriptions);
        assert_eq!(manager.particles.live_count(id), 1024);
        let other = manager.create_instance(
            manager.get(id).unwrap().descriptor,
            VfxSpawnParams {
                pos: Vec2::zero(),
                scale: 1.0,
                direction: Vec2::new(1.0, 0.0),
                lifetime: 1.0,
            },
            Vec::new(),
        );
        manager.get_mut(other).unwrap().state = VfxInstanceState::Active;
        manager.simulate(0.5, &descriptions);
        assert_eq!(
            manager.get(other).unwrap().state,
            VfxInstanceState::Finished
        );
        manager.simulate(0.25, &descriptions);
        assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
    }

    #[test]
    fn burst_fires_once_after_delay_even_when_frame_crosses_it() {
        let emission = VfxEmission {
            mode: VfxEmissionMode::Burst,
            count: 24,
            delay_seconds: 0.5,
        };
        let mut state = EmissionState::default();
        assert_eq!(state.advance(&emission, 0.25, 100.0), 0);
        assert_eq!(state.advance(&emission, 0.5, 100.0), 24);
        assert_eq!(state.advance(&emission, 10.0, 100.0), 0);
        let immediate = VfxEmission {
            delay_seconds: 0.0,
            ..emission
        };
        let mut state = EmissionState::default();
        assert_eq!(state.advance(&immediate, 0.0, 100.0), 24);
        assert_eq!(state.advance(&immediate, 0.0, 100.0), 0);
    }

    #[test]
    fn continuous_keeps_fractions_and_counts_only_time_after_delay() {
        let emission = VfxEmission {
            mode: VfxEmissionMode::Continuous,
            count: 3,
            delay_seconds: 0.5,
        };
        let mut state = EmissionState::default();
        assert_eq!(state.advance(&emission, 0.75, 100.0), 0);
        assert_eq!(state.advance(&emission, 0.25, 100.0), 1);
        assert_eq!(state.advance(&emission, 0.5, 100.0), 2);
        assert_eq!(state.advance(&emission, 0.0, 100.0), 0);
    }

    #[test]
    fn continuous_count_is_independent_of_frame_partition() {
        let emission = VfxEmission {
            mode: VfxEmissionMode::Continuous,
            count: 10,
            delay_seconds: 0.5,
        };
        let mut coarse = EmissionState::default();
        let expected = coarse.advance(&emission, 2.0, 100.0);
        let mut fine = EmissionState::default();
        let actual: u32 = (0..16).map(|_| fine.advance(&emission, 0.125, 100.0)).sum();
        assert_eq!(actual, expected);
        assert_eq!(actual, 15);
    }

    #[test]
    fn continuous_duration_cap_is_independent_of_frame_partition() {
        let emission = VfxEmission {
            mode: VfxEmissionMode::Continuous,
            count: 10,
            delay_seconds: 0.5,
        };
        let mut coarse = EmissionState::default();
        assert_eq!(coarse.advance(&emission, 4.0, 1.0), 10);
        assert!(coarse.finished(&emission, 1.0));
        assert_eq!(coarse.advance(&emission, 1.0, 1.0), 0);
        let mut fine = EmissionState::default();
        let count: u32 = (0..32).map(|_| fine.advance(&emission, 0.125, 1.0)).sum();
        assert_eq!(count, 10);
        assert!(fine.finished(&emission, 1.0));
    }

    #[test]
    fn waiting_and_finishing_instances_do_not_emit() {
        let description = crate::vfx::parser::parse_effect(include_str!(
            "../../../assets/config/vfx/exemples_vfx/particle.json"
        ))
        .unwrap();
        let mut descriptions = AssetRegister::new();
        let descriptor = descriptions.insert(description);
        let mut manager = VfxInstanceManager::new();
        let id = manager.create_instance(
            descriptor,
            VfxSpawnParams {
                pos: Vec2::zero(),
                scale: 1.0,
                direction: Vec2::new(1.0, 0.0),
                lifetime: 1.0,
            },
            Vec::new(),
        );
        manager.simulate(10.0, &descriptions);
        assert_eq!(manager.get(id).unwrap().age, 0.0);
        assert!(!manager.get(id).unwrap().emission.burst_emitted);
        manager.get_mut(id).unwrap().state = VfxInstanceState::Active;
        manager.simulate(0.0, &descriptions);
        assert!(manager.get(id).unwrap().emission.burst_emitted);
        manager.get_mut(id).unwrap().state = VfxInstanceState::Finishing;
        let elapsed = manager.get(id).unwrap().emission.elapsed;
        manager.simulate(0.5, &descriptions);
        assert_eq!(manager.get(id).unwrap().emission.elapsed, elapsed);
    }

    #[test]
    fn particle_velocity_supports_constant_ranges_and_degrees() {
        let mut rng = StdRng::seed_from_u64(1);
        let v = sample_velocity(&mut rng, [90.0, 90.0], [80.0, 80.0]);
        assert!(v.x.abs() < 0.0001);
        assert!((v.y - 80.0).abs() < 0.0001);
    }

    #[test]
    fn particle_positions_cover_disk_area_and_boundary() {
        let mut rng = StdRng::seed_from_u64(2);
        let mut squared_sum = 0.0;
        for _ in 0..10000 {
            let p = sample_disk(&mut rng, 2.0, &VfxSpawnLocation::Interior);
            let squared = p.x * p.x + p.y * p.y;
            assert!(squared <= 4.00001);
            squared_sum += squared;
            let p = sample_disk(&mut rng, 2.0, &VfxSpawnLocation::Boundary);
            assert!((p.x * p.x + p.y * p.y - 4.0).abs() < 0.00001);
        }
        // Une distribution uniforme en surface donne E[r²] = R² / 2.
        assert!((squared_sum / 10000.0 - 2.0).abs() < 0.05);
    }
}
