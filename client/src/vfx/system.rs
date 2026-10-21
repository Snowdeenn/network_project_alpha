use std::collections::HashMap;
use std::sync::Arc;

use math::Vec2;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use utils::{Arena, Id};

use super::api::VfxApi;
use super::dag::{VfxGraph, VfxNodeEffect};
use super::instance::{VfxInstanceId, VfxInstanceManager, VfxInstanceState, VfxSpawnParams};
use super::parser::r#struct::{VfxEffectDescriptor, VfxEmissionMode, VfxFamily};
use super::registry::{DefinitionKind, Priority, VfxDefinition, VfxRegistry};
use super::request::*;

const STEP: f32 = 1.0 / 60.0;
const MAX_STEPS: usize = 8;
const KEY_TTL: f64 = 10.0;
const MAX_KEYS: usize = 2048;

#[derive(Clone, Copy)]
pub struct VfxBudgets {
    pub queued_requests: usize,
    pub requests_per_frame: usize,
    pub instances: usize,
    pub components: usize,
    pub particles: usize,
    pub elements: usize,
    pub trail_points: usize,
}

impl Default for VfxBudgets {
    fn default() -> Self {
        Self {
            queued_requests: 512,
            requests_per_frame: 256,
            instances: 256,
            components: 1024,
            particles: 1024,
            elements: 4096,
            trail_points: 4096,
        }
    }
}

#[derive(Default)]
pub struct VfxDiagnostics {
    pub spawned: u64,
    pub rejected: u64,
    pub queue_rejected: u64,
    pub duplicates: u64,
    pub stale_handles: u64,
    pub invalid_changes: u64,
    pub active: usize,
    pub peak: usize,
    pub discarded_seconds: f64,
    pub unsupported_components: u64,
}

/// Le contenu reste privé ; l'arène existante protège les handles périmés.
pub struct RunningEffect {
    definition: Arc<VfxDefinition>,
    components: Vec<VfxInstanceId>,
    graph: VfxGraph,
    attachment: Option<Attachment>,
    reserved: Reservation,
    stopping: bool,
    looping: bool,
}

#[derive(Default, Clone, Copy)]
struct Reservation {
    particles: usize,
    elements: usize,
    trail_points: usize,
}

pub struct VfxSystem {
    pub registry: VfxRegistry,
    pub diagnostics: VfxDiagnostics,
    pub(super) instances: VfxInstanceManager,
    pub(super) running: Arena<RunningEffect>,
    api: VfxApi,
    budgets: VfxBudgets,
    accumulator: f64,
    time: f64,
    events: HashMap<VisualEventKey, (f64, SpawnResult)>,
    predictions: HashMap<PredictionKey, (f64, SpawnResult)>,
    random: StdRng,
}

impl Default for VfxSystem {
    fn default() -> Self {
        Self::new(VfxBudgets::default())
    }
}

impl VfxSystem {
    pub fn new(budgets: VfxBudgets) -> Self {
        Self::with_seed(budgets, rand::random())
    }

    pub fn with_seed(mut budgets: VfxBudgets, seed: u64) -> Self {
        // Le pool existant possède 1024 slots : la configuration ne peut pas
        // promettre davantage de particules que la simulation n'en stocke.
        budgets.particles = budgets.particles.min(1024);
        Self {
            registry: VfxRegistry::default(),
            diagnostics: VfxDiagnostics::default(),
            instances: VfxInstanceManager::new(),
            running: Arena::new(),
            api: VfxApi::new(budgets.queued_requests),
            budgets,
            accumulator: 0.0,
            time: 0.0,
            events: HashMap::new(),
            predictions: HashMap::new(),
            random: StdRng::seed_from_u64(seed),
        }
    }

    pub fn api(&self) -> VfxApi {
        self.api.clone()
    }

    pub fn is_alive(&self, handle: VfxHandle) -> bool {
        self.running.get(handle).is_some()
    }

    /// Appeler après la mise à jour du monde. La vue ne vit que durant cet appel.
    /// Les requêtes prennent effet avant les sous-pas de simulation.
    pub fn update(&mut self, dt: f32, entity_position: impl Fn(u64) -> Option<Vec2>) {
        if !dt.is_finite() || dt < 0.0 {
            return;
        }
        self.time += f64::from(dt);
        self.events
            .retain(|_, (time, _)| self.time - *time < KEY_TTL);
        self.predictions.retain(|_, (time, result)| {
            self.time - *time < KEY_TTL || matches!(result, SpawnResult::Spawned(handle) if self.running.get(*handle).is_some())
        });
        for _ in 0..self.budgets.requests_per_frame {
            let request = self.api.requests.borrow_mut().queue.pop_front();
            let Some(request) = request else {
                break;
            };
            self.process_request(request);
        }
        self.diagnostics.queue_rejected = self.api.requests.borrow().rejected;
        self.resolve_attachments(&entity_position);

        self.accumulator += f64::from(dt);
        let step = f64::from(STEP);
        let steps = (self.accumulator / step).floor().min(MAX_STEPS as f64) as usize;
        for _ in 0..steps {
            for effect in self.running.iter_mut() {
                if !effect.stopping {
                    self.instances.run(&mut effect.graph, STEP);
                }
            }
            self.instances.simulate(STEP, &self.registry.assets);
            self.accumulator -= step;
            self.collect_finished();
        }
        // Une pause ou surcharge ne crée pas une spirale de rattrapage.
        if self.accumulator >= step {
            let discarded = (self.accumulator / step).floor() * step;
            self.accumulator -= discarded;
            self.diagnostics.discarded_seconds += discarded;
        }
        self.registry.collect_retired();
        self.diagnostics.active = self.running.iter().count();
        self.diagnostics.peak = self.diagnostics.peak.max(self.diagnostics.active);
    }

    fn process_request(&mut self, request: VfxRequest) {
        match request {
            VfxRequest::Spawn {
                ticket,
                effect,
                params,
                options,
            } => {
                let result = self.spawn(&effect, params, options);
                if let Some(ticket) = ticket {
                    self.api
                        .requests
                        .borrow_mut()
                        .results
                        .insert(ticket, Some(result));
                }
            }
            VfxRequest::Update(ticket, handle, change) => {
                let result = self.change(handle, change);
                self.api
                    .requests
                    .borrow_mut()
                    .command_results
                    .insert(ticket, Some(result));
            }
            VfxRequest::Stop(ticket, handle, mode) => {
                let result = if self.stop(handle, mode) {
                    Ok(())
                } else {
                    Err(VfxCommandError::StaleHandle)
                };
                self.api
                    .requests
                    .borrow_mut()
                    .command_results
                    .insert(ticket, Some(result));
            }
            VfxRequest::Reconcile {
                prediction,
                confirmed,
            } => {
                if !confirmed {
                    if let Some((_, SpawnResult::Spawned(handle))) =
                        self.predictions.get(&prediction).copied()
                    {
                        self.stop(handle, StopMode::Immediate);
                    }
                    self.remember_prediction(
                        prediction,
                        SpawnResult::Rejected(VfxRejection::Cancelled),
                    );
                }
                // Confirmer conserve simplement l'association existante. Un
                // spawn autoritaire portant la même clé résout le même handle.
            }
            VfxRequest::StopNamed(name) => {
                let handles: Vec<_> = self
                    .running
                    .iter_with_ids()
                    .filter_map(|(handle, effect)| {
                        (effect.definition.name == name).then_some(handle)
                    })
                    .collect();
                for handle in handles {
                    self.stop(handle, StopMode::Immediate);
                }
            }
        }
    }

    pub fn change(&mut self, handle: VfxHandle, change: VfxChange) -> Result<(), VfxCommandError> {
        let Some(effect) = self.running.get(handle) else {
            self.diagnostics.stale_handles += 1;
            return Err(VfxCommandError::StaleHandle);
        };
        let valid = match &change {
            VfxChange::Position(pos) | VfxChange::Direction(pos) => valid_position(*pos),
            VfxChange::Scale(scale) => scale.is_finite() && *scale >= 0.0,
        };
        if !valid {
            self.diagnostics.invalid_changes += 1;
            return Err(VfxCommandError::InvalidParams);
        }
        for &id in &effect.components {
            let instance = self.instances.get_mut(id).unwrap();
            match change {
                VfxChange::Position(pos) => instance.params.pos = pos,
                VfxChange::Direction(direction) => instance.params.direction = direction,
                VfxChange::Scale(scale) => instance.params.scale = scale,
            }
        }
        Ok(())
    }

    fn spawn(&mut self, name: &str, params: VfxSpawnParams, options: SpawnOptions) -> SpawnResult {
        if !valid_params(&params)
            || options
                .attachment
                .is_some_and(|attachment| !valid_position(attachment.offset))
        {
            self.diagnostics.rejected += 1;
            return SpawnResult::Rejected(VfxRejection::InvalidParams);
        }
        let previous = options
            .prediction
            .and_then(|key| self.predictions.get(&key))
            .or_else(|| options.event_key.and_then(|key| self.events.get(&key)));
        if let Some((_, result)) = previous.copied() {
            self.diagnostics.duplicates += 1;
            // Relier aussi la clé du message serveur à l'occurrence prédite.
            if let Some(key) = options.event_key {
                remember(&mut self.events, key, self.time, result);
            }
            if let Some(key) = options.prediction {
                self.remember_prediction(key, result);
            }
            return result;
        }
        if options.refresh {
            let previous = self.running.iter_with_ids().find_map(|(handle, effect)| {
                (effect.definition.name == name
                    && effect.components.first().is_some_and(|&id| {
                        self.instances.get(id).unwrap().params.entity == params.entity
                    }))
                .then_some(handle)
            });
            if let Some(handle) = previous {
                let effect = self.running.get_mut(handle).unwrap();
                let manual_trail = !effect.stopping
                    && effect.components.iter().all(|&id| {
                        let instance = self.instances.get(id).unwrap();
                        matches!(&self.registry.assets.get(instance.descriptor).unwrap().family,
                        VfxFamily::Trail { logical, .. } if logical.emission.manual)
                    });
                if manual_trail {
                    for &id in &effect.components {
                        let instance = self.instances.get_mut(id).unwrap();
                        let VfxFamily::Trail { logical, .. } = &self
                            .registry
                            .assets
                            .get(instance.descriptor)
                            .unwrap()
                            .family
                        else {
                            unreachable!()
                        };
                        instance.params = params;
                        instance.state = VfxInstanceState::Active;
                        instance.trail_samples.push(params.pos);
                        let max = logical.evolution.max_points as usize;
                        if instance.trail_samples.len() > max {
                            instance
                                .trail_samples
                                .drain(..instance.trail_samples.len() - max);
                        }
                    }
                    let result = SpawnResult::Spawned(handle);
                    if let Some(key) = options.event_key {
                        remember(&mut self.events, key, self.time, result);
                    }
                    if let Some(key) = options.prediction {
                        self.remember_prediction(key, result);
                    }
                    return result;
                }
                self.stop(handle, StopMode::Immediate);
            }
        }
        let result = self.admit(name, params, options.attachment, options.looping);
        if let Some(key) = options.event_key {
            remember(&mut self.events, key, self.time, result);
        }
        if let Some(key) = options.prediction {
            self.remember_prediction(key, result);
        }
        if matches!(result, SpawnResult::Rejected(_)) {
            self.diagnostics.rejected += 1;
        }
        result
    }

    fn admit(
        &mut self,
        name: &str,
        params: VfxSpawnParams,
        attachment: Option<Attachment>,
        looping: bool,
    ) -> SpawnResult {
        if !valid_params(&params) || attachment.is_some_and(|a| !valid_position(a.offset)) {
            return SpawnResult::Rejected(VfxRejection::InvalidParams);
        }
        let Some(definition) = self.registry.get(name).cloned() else {
            tracing::warn!(effect = name, "Unknown VFX effect");
            return SpawnResult::Rejected(VfxRejection::UnknownEffect);
        };
        let mut leaves = Vec::new();
        flatten(&definition, 0.0, &mut leaves);
        let count = self.running.iter().count();
        let components: usize = self.running.iter().map(|e| e.components.len()).sum();
        let limit = if definition.priority == Priority::Low {
            self.budgets.instances * 4 / 5
        } else {
            self.budgets.instances
        };
        if count >= limit || components + leaves.len() > self.budgets.components {
            return SpawnResult::Rejected(VfxRejection::InstanceBudget);
        }
        let mut reserved = Reservation::default();
        for &(id, _) in &leaves {
            let budget = reservation(self.registry.assets.get(id).unwrap(), params.lifetime);
            reserved.particles += budget.particles;
            reserved.elements += budget.elements;
            reserved.trail_points += budget.trail_points;
        }
        let mut used = reserved;
        for effect in self.running.iter() {
            used.particles += effect.reserved.particles;
            used.elements += effect.reserved.elements;
            used.trail_points += effect.reserved.trail_points;
        }
        if used.particles > self.budgets.particles
            || used.elements > self.budgets.elements
            || used.trail_points > self.budgets.trail_points
        {
            return SpawnResult::Rejected(VfxRejection::ElementBudget);
        }
        let mut graph = VfxGraph::new(name.to_owned());
        let mut ids = Vec::new();
        for (id, delay) in leaves {
            let unsupported = match &self.registry.assets.get(id).unwrap().family {
                VfxFamily::Light { .. } => true,
                _ => false,
            };
            if unsupported {
                self.diagnostics.unsupported_components += 1;
            }
            let instance = self.instances.create_instance(id, params, Vec::new());
            self.instances.set_seed(instance, self.random.random());
            graph.add_delayed_node(VfxNodeEffect::Effect(instance), delay);
            ids.push(instance);
        }
        self.instances.run(&mut graph, 0.0);
        let handle = self.running.insert(RunningEffect {
            definition,
            components: ids,
            graph,
            attachment,
            reserved,
            stopping: false,
            looping,
        });
        self.diagnostics.spawned += 1;
        self.diagnostics.peak = self.diagnostics.peak.max(count + 1);
        SpawnResult::Spawned(handle)
    }

    /// Retour explicite pour le propriétaire ; les commandes asynchrones
    /// périmées sont également comptées dans diagnostics.stale_handles.
    pub fn stop(&mut self, handle: VfxHandle, mode: StopMode) -> bool {
        let Some(effect) = self.running.get_mut(handle) else {
            self.diagnostics.stale_handles += 1;
            return false;
        };
        match mode {
            StopMode::Immediate => self.remove(handle),
            StopMode::Drain => {
                effect.stopping = true;
                for &id in &effect.components {
                    let instance = self.instances.get_mut(id).unwrap();
                    instance.state = match instance.state {
                        VfxInstanceState::Waiting => VfxInstanceState::Finished,
                        VfxInstanceState::Active => VfxInstanceState::Finishing,
                        state => state,
                    };
                }
            }
        }
        true
    }

    fn resolve_attachments(&mut self, position: &impl Fn(u64) -> Option<Vec2>) {
        let handles: Vec<_> = self.running.iter_with_ids().map(|(id, _)| id).collect();
        for handle in handles {
            let effect = self.running.get_mut(handle).unwrap();
            let Some(attachment) = effect.attachment else {
                if let Some(entity) = effect
                    .components
                    .first()
                    .and_then(|&id| self.instances.get(id).unwrap().params.entity)
                    && position(entity)
                        .filter(|pos| valid_position(*pos))
                        .is_none()
                {
                    self.stop(handle, StopMode::Drain);
                }
                continue;
            };
            if let Some(pos) = position(attachment.entity).filter(|pos| valid_position(*pos)) {
                for &id in &effect.components {
                    self.instances.get_mut(id).unwrap().params.pos = pos + attachment.offset;
                }
            } else {
                effect.attachment = None;
                match attachment.on_missing {
                    MissingOwner::Drain => {
                        self.stop(handle, StopMode::Drain);
                    }
                    MissingOwner::Immediate => {
                        self.stop(handle, StopMode::Immediate);
                    }
                    MissingOwner::Detach => {}
                }
            }
        }
    }

    fn collect_finished(&mut self) {
        let finished: Vec<_> = self
            .running
            .iter_with_ids()
            .filter_map(|(handle, effect)| {
                effect
                    .components
                    .iter()
                    .all(|&id| self.instances.get(id).unwrap().state == VfxInstanceState::Finished)
                    .then_some(handle)
            })
            .collect();
        for handle in finished {
            let effect = self.running.get_mut(handle).unwrap();
            if effect.looping && !effect.stopping {
                let params = self.instances.get(effect.components[0]).unwrap().params;
                for id in effect.components.drain(..) {
                    self.instances.destroy_instance(id);
                }
                let mut leaves = Vec::new();
                flatten(&effect.definition, 0.0, &mut leaves);
                effect.graph = VfxGraph::new(effect.definition.name.clone());
                for (descriptor, delay) in leaves {
                    let id = self
                        .instances
                        .create_instance(descriptor, params, Vec::new());
                    self.instances.set_seed(id, self.random.random());
                    effect
                        .graph
                        .add_delayed_node(VfxNodeEffect::Effect(id), delay);
                    effect.components.push(id);
                }
                self.instances.run(&mut effect.graph, 0.0);
            } else {
                self.remove(handle);
            }
        }
    }

    fn remove(&mut self, handle: VfxHandle) {
        // Arena::remove doit être précédé d'un get : ne pas libérer deux fois.
        if self.running.get(handle).is_none() {
            return;
        }
        let effect = self.running.remove(handle).unwrap();
        for id in effect.components {
            self.instances.destroy_instance(id);
        }
    }

    fn remember_prediction(&mut self, key: PredictionKey, result: SpawnResult) {
        remember(&mut self.predictions, key, self.time, result);
    }

    pub fn take_camera_trauma(&mut self) -> f32 {
        self.instances.take_camera_trauma()
    }

    pub(super) fn components(&self) -> impl Iterator<Item = (&VfxDefinition, &[VfxInstanceId])> {
        self.running
            .iter()
            .map(|effect| (effect.definition.as_ref(), effect.components.as_slice()))
    }
}

fn flatten(
    definition: &VfxDefinition,
    delay: f32,
    leaves: &mut Vec<(Id<VfxEffectDescriptor>, f32)>,
) {
    match &definition.kind {
        DefinitionKind::Effect(id) => leaves.push((*id, delay)),
        DefinitionKind::Composite(children) => {
            for (child, offset) in children {
                flatten(child, delay + offset, leaves);
            }
        }
    }
}

fn reservation(effect: &VfxEffectDescriptor, duration: f32) -> Reservation {
    let count = |emission: &super::parser::r#struct::VfxEmission| match emission.mode {
        VfxEmissionMode::Burst => emission.count as usize,
        VfxEmissionMode::Continuous => (emission.count as f32 * duration).ceil() as usize,
    };
    match &effect.family {
        VfxFamily::Particle { logical, .. } => Reservation {
            particles: count(&logical.emission),
            ..Reservation::default()
        },
        VfxFamily::Mesh { logical, .. } => Reservation {
            elements: count(&logical.emission),
            ..Reservation::default()
        },
        VfxFamily::Decal { logical, .. } => Reservation {
            elements: count(&logical.emission),
            ..Reservation::default()
        },
        VfxFamily::Light { logical, .. } => Reservation {
            elements: count(&logical.emission),
            ..Reservation::default()
        },
        VfxFamily::Trail { logical, .. } => Reservation {
            trail_points: logical.evolution.max_points as usize,
            ..Reservation::default()
        },
        VfxFamily::PostProcess { .. } | VfxFamily::Feedback { .. } => Reservation {
            elements: 1,
            ..Reservation::default()
        },
    }
}

fn remember<K: Eq + std::hash::Hash + Copy>(
    map: &mut HashMap<K, (f64, SpawnResult)>,
    key: K,
    time: f64,
    result: SpawnResult,
) {
    if map.len() >= MAX_KEYS && !map.contains_key(&key) {
        if let Some(oldest) = map
            .iter()
            .min_by(|a, b| a.1.0.total_cmp(&b.1.0))
            .map(|(key, _)| *key)
        {
            map.remove(&oldest);
        }
    }
    map.insert(key, (time, result));
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod tests;
