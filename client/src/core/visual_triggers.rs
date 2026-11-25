//! Gameplay adapters emit triggers; resolved requests retain the complete VFX API.
use super::visual_assets::VisualAssets;
use crate::vfx::{
    Attachment, MissingOwner, SpawnOptions, SpawnResult, SpawnTicket, StopMode, VfxHandle,
    VfxSpawnParams, VfxSystem,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use utils::{
    math::Vec2,
    protocol::{EntityState, VisualEvent, VisualProfileId, VisualTrigger},
};

fn max_delay() -> Option<f32> {
    Some(0.5)
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentBinding {
    #[serde(default)]
    pub offset: Vec2,
    pub on_missing: MissingOwner,
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpawnBindings {
    pub position: bool,
    pub direction: bool,
    pub entity: bool,
    pub attachment: Option<AttachmentBinding>,
}

impl Default for SpawnBindings {
    fn default() -> Self {
        Self {
            position: true,
            direction: true,
            entity: false,
            attachment: None,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VisualAction {
    Spawn {
        effect: String,
        #[serde(default)]
        params: VfxSpawnParams,
        #[serde(default)]
        options: SpawnOptions,
        #[serde(default)]
        bindings: SpawnBindings,
        #[serde(default)]
        track_as: Option<String>,
        #[serde(default = "max_delay")]
        max_delay_seconds: Option<f32>,
    },
    Stop {
        tracked: String,
        mode: StopMode,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualRule {
    pub trigger: VisualTrigger,
    pub actions: Vec<VisualAction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualProfile {
    pub id: VisualProfileId,
    pub name: String,
    pub rules: Vec<VisualRule>,
}

pub struct VisualProfiles(HashMap<VisualProfileId, VisualProfile>);

impl VisualProfiles {
    pub fn id(&self, name: &str) -> Option<VisualProfileId> {
        self.0
            .values()
            .find(|profile| profile.name == name)
            .map(|profile| profile.id)
    }
    pub fn parse(source: &str) -> std::io::Result<Self> {
        let profiles: Vec<VisualProfile> = serde_json::from_str(source)?;
        let mut result = HashMap::new();
        for profile in profiles {
            if profile.name.trim().is_empty()
                || result.contains_key(&profile.id)
                || result
                    .values()
                    .any(|other: &VisualProfile| other.name == profile.name)
            {
                return Err(std::io::Error::other("empty or duplicate visual profile"));
            }
            for rule in &profile.rules {
                for action in &rule.actions {
                    if let VisualAction::Spawn {
                        effect,
                        max_delay_seconds,
                        ..
                    } = action
                    {
                        if effect.trim().is_empty()
                            || max_delay_seconds.is_some_and(|v| !v.is_finite() || v < 0.0)
                        {
                            return Err(std::io::Error::other("invalid visual spawn rule"));
                        }
                    }
                }
            }
            result.insert(profile.id, profile);
        }
        Ok(Self(result))
    }

    pub fn bundled() -> std::io::Result<Self> {
        Self::parse(include_str!("../../../assets/config/visual_profiles.json"))
    }

    pub fn preload(&self, id: VisualProfileId, assets: &mut VisualAssets) {
        if let Some(profile) = self.0.get(&id) {
            for rule in &profile.rules {
                for action in &rule.actions {
                    if let VisualAction::Spawn { effect, .. } = action {
                        assets.ensure_loaded(effect);
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct VisualContext {
    /// Stable scope for named occurrences, usually the source entity ID.
    pub scope: u64,
    pub entity: Option<u64>,
    pub position: Vec2,
    pub direction: Vec2,
}

type TrackKey = (u64, VisualProfileId, String);

pub struct ResolvedVfxSpawn {
    pub effect: String,
    pub params: VfxSpawnParams,
    pub options: SpawnOptions,
    pub track_as: Option<String>,
    pub scope: u64,
    pub profile: VisualProfileId,
    pub owner: Option<u64>,
    pub max_delay_seconds: Option<f32>,
}

struct PendingSpawn {
    request: ResolvedVfxSpawn,
    queued_at: f64,
}
struct Occurrence {
    track: Option<TrackKey>,
    owner: Option<u64>,
    ticket: Option<SpawnTicket>,
    handle: Option<VfxHandle>,
    stop: Option<StopMode>,
    on_missing: MissingOwner,
}

#[derive(Default)]
pub struct VisualTriggers {
    time: f64,
    known: HashMap<u64, (VisualProfileId, VisualContext)>,
    pending: VecDeque<PendingSpawn>,
    active: Vec<Occurrence>,
    seen_events: HashMap<u64, f64>,
}

impl VisualTriggers {
    /// Customization happens after resolution and before queuing/loading.
    pub fn emit_with(
        &mut self,
        profiles: &VisualProfiles,
        profile_id: VisualProfileId,
        trigger: &VisualTrigger,
        context: &VisualContext,
        mut customize: impl FnMut(&mut ResolvedVfxSpawn),
    ) {
        let Some(profile) = profiles.0.get(&profile_id) else {
            tracing::warn!(profile_id, "Unknown visual profile");
            return;
        };
        for rule in &profile.rules {
            if rule.trigger != *trigger {
                continue;
            }
            for action in &rule.actions {
                match action {
                    VisualAction::Stop { tracked, mode } => {
                        self.stop_tracked(&(context.scope, profile_id, tracked.clone()), *mode);
                    }
                    VisualAction::Spawn {
                        effect,
                        params,
                        options,
                        bindings,
                        track_as,
                        max_delay_seconds,
                    } => {
                        let mut request = ResolvedVfxSpawn {
                            effect: effect.clone(),
                            params: *params,
                            options: options.clone(),
                            track_as: track_as.clone(),
                            scope: context.scope,
                            profile: profile_id,
                            owner: if *trigger == VisualTrigger::Spawn {
                                context.entity
                            } else {
                                None
                            },
                            max_delay_seconds: *max_delay_seconds,
                        };
                        if bindings.position {
                            request.params.pos = context.position;
                        }
                        if bindings.direction {
                            request.params.direction = context.direction;
                        }
                        if bindings.entity {
                            request.params.entity = context.entity;
                        }
                        if let Some(binding) = &bindings.attachment {
                            let Some(entity) = context.entity else {
                                tracing::warn!(effect, "Attachment binding requires an entity");
                                continue;
                            };
                            request.options.attachment = Some(Attachment {
                                entity,
                                offset: binding.offset,
                                on_missing: binding.on_missing,
                            });
                        }
                        customize(&mut request);
                        self.enqueue(request);
                    }
                }
            }
        }
    }

    pub fn emit(
        &mut self,
        profiles: &VisualProfiles,
        profile: VisualProfileId,
        trigger: &VisualTrigger,
        context: &VisualContext,
    ) {
        self.emit_with(profiles, profile, trigger, context, |_| {});
    }

    /// Also accepts completely custom requests without a profile rule.
    pub fn enqueue(&mut self, request: ResolvedVfxSpawn) {
        if let Some(name) = &request.track_as {
            self.stop_tracked(
                &(request.scope, request.profile, name.clone()),
                StopMode::Immediate,
            );
        }
        self.pending.push_back(PendingSpawn {
            request,
            queued_at: self.time,
        });
    }

    fn stop_tracked(&mut self, key: &TrackKey, mode: StopMode) {
        self.pending.retain(|pending| {
            let request = &pending.request;
            !(request.scope == key.0
                && request.profile == key.1
                && request.track_as.as_ref() == Some(&key.2))
        });
        for occurrence in &mut self.active {
            if occurrence.track.as_ref() == Some(key) {
                occurrence.stop = Some(mode);
            }
        }
    }

    pub fn event(&mut self, profiles: &VisualProfiles, event: &VisualEvent) {
        if self.seen_events.contains_key(&event.id) {
            return;
        }
        self.seen_events.insert(event.id, self.time);
        self.emit(
            profiles,
            event.profile,
            &event.trigger,
            &VisualContext {
                scope: event.entity.unwrap_or(event.id),
                entity: event.entity,
                position: Vec2::new(event.position[0], event.position[1]),
                direction: Vec2::new(event.direction[0], event.direction[1]),
            },
        );
    }

    /// Call for every received snapshot, including intermediate snapshots in a batch.
    pub fn sync(&mut self, profiles: &VisualProfiles, entities: &[EntityState]) {
        let present: HashSet<_> = entities.iter().map(|entity| entity.entity_id).collect();
        let removed: Vec<_> = self
            .known
            .keys()
            .filter(|id| !present.contains(id))
            .copied()
            .collect();
        for id in removed {
            self.remove_entity(profiles, id);
        }
        for entity in entities {
            let id = entity.entity_id;
            if self.known.get(&id).map(|known| known.0) != entity.visual_profile {
                self.remove_entity(profiles, id);
            }
            let Some(profile) = entity.visual_profile else {
                continue;
            };
            let context = VisualContext {
                scope: id,
                entity: Some(id),
                position: Vec2::new(entity.position[0], entity.position[1]),
                direction: Vec2::new(entity.direction[0], entity.direction[1]),
            };
            if !self.known.contains_key(&id) {
                self.emit(profiles, profile, &VisualTrigger::Spawn, &context);
            }
            self.known.insert(id, (profile, context));
        }
    }

    fn remove_entity(&mut self, profiles: &VisualProfiles, id: u64) {
        if let Some((profile, context)) = self.known.remove(&id) {
            self.pending
                .retain(|pending| pending.request.owner != Some(id));
            for occurrence in &mut self.active {
                if occurrence.owner == Some(id) {
                    match occurrence.on_missing {
                        MissingOwner::Immediate => occurrence.stop = Some(StopMode::Immediate),
                        MissingOwner::Drain => occurrence.stop = Some(StopMode::Drain),
                        MissingOwner::Detach => occurrence.owner = None,
                    }
                }
            }
            self.emit(profiles, profile, &VisualTrigger::Despawn, &context);
        }
    }

    pub fn update(&mut self, dt: f32, assets: &mut VisualAssets, system: &mut VfxSystem) {
        if dt.is_finite() && dt > 0.0 {
            self.time += f64::from(dt);
        }
        self.seen_events.retain(|_, time| self.time - *time < 60.0);
        let api = system.api();
        self.active.retain_mut(|occurrence| {
            if let Some(ticket) = occurrence.ticket {
                if occurrence.stop.is_some() {
                    api.cancel_pending(ticket);
                }
                match api.take_result(ticket) {
                    None => return true,
                    Some(SpawnResult::Rejected(reason)) => {
                        tracing::debug!(?reason, "Visual spawn rejected");
                        return false;
                    }
                    Some(SpawnResult::Spawned(handle)) => {
                        occurrence.ticket = None;
                        occurrence.handle = Some(handle);
                    }
                }
            }
            let Some(handle) = occurrence.handle else {
                return false;
            };
            if let Some(mode) = occurrence.stop.take() {
                system.stop(handle, mode);
            }
            system.is_alive(handle)
        });
        let count = self.pending.len();
        for _ in 0..count {
            let Some(pending) = self.pending.pop_front() else {
                break;
            };
            let request = &pending.request;
            if request
                .max_delay_seconds
                .is_some_and(|ttl| self.time - pending.queued_at > f64::from(ttl))
            {
                continue;
            }
            if request
                .owner
                .is_some_and(|owner| !self.known.contains_key(&owner))
            {
                continue;
            }
            assets.ensure_loaded(&request.effect);
            if system.registry.get(&request.effect).is_none() {
                self.pending.push_back(pending);
                continue;
            }
            let track = request
                .track_as
                .as_ref()
                .map(|name| (request.scope, request.profile, name.clone()));
            let PendingSpawn { request, queued_at } = pending;
            match api.spawn_with(&request.effect, request.params, request.options.clone()) {
                Ok(ticket) => self.active.push(Occurrence {
                    track,
                    owner: request.owner,
                    ticket: Some(ticket),
                    handle: None,
                    stop: None,
                    on_missing: request
                        .options
                        .attachment
                        .map_or(MissingOwner::Detach, |a| a.on_missing),
                }),
                Err(_) => self.pending.push_back(PendingSpawn { request, queued_at }),
            }
        }
    }

    pub fn clear(&mut self, system: &mut VfxSystem) {
        self.pending.clear();
        let api = system.api();
        for occurrence in self.active.drain(..) {
            let mut handle = occurrence.handle;
            if let Some(ticket) = occurrence.ticket {
                api.cancel_pending(ticket);
                if let Some(SpawnResult::Spawned(spawned)) = api.take_result(ticket) {
                    handle = Some(spawned);
                }
            }
            if let Some(handle) = handle {
                system.stop(handle, StopMode::Immediate);
            }
        }
        self.known.clear();
        self.seen_events.clear();
        self.time = 0.0;
    }
}

#[cfg(test)]
#[path = "visual_tests.rs"]
mod tests;
