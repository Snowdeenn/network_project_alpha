use super::*;
use crate::vfx::{AssetLoader, PredictionKey, VfxBudgets, VisualEventKey};
use std::time::{Duration, Instant};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

async fn load(
    assets: &mut VisualAssets,
    loader: &mut AssetLoader,
    system: &mut VfxSystem,
    names: &[&str],
) {
    for name in names {
        assets.ensure_loaded(name);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while names.iter().any(|name| system.registry.get(name).is_none()) {
        assert!(Instant::now() < deadline, "loading timed out");
        for name in names {
            assert!(assets.error(name).is_none(), "{:?}", assets.error(name));
        }
        loader.run_request().await;
        assets.publish(loader, &mut system.registry);
        tokio::task::yield_now().await;
    }
}

fn entity(id: u64, profile: u32) -> EntityState {
    EntityState {
        entity_id: id,
        visual_profile: Some(profile),
        direction: [0.0, 1.0],
        position: [100.0, 200.0],
        health: 0.0,
        max_health: 0.0,
        entity_kind: utils::protocol::EntityKind::Projectile,
    }
}

fn context() -> VisualContext {
    VisualContext {
        scope: 42,
        entity: Some(42),
        position: Vec2::new(100.0, 200.0),
        direction: Vec2::new(0.0, 1.0),
    }
}

#[test]
fn different_projectiles_load_once_follow_owners_and_stop_on_despawn() {
    runtime().block_on(async {
        let profiles = VisualProfiles::bundled().unwrap();
        let mut assets = VisualAssets::bundled().unwrap();
        let mut loader = AssetLoader::new();
        let mut system = VfxSystem::default();
        let mut triggers = VisualTriggers::default();
        let entities = [entity(42, 1), entity(43, 2)];
        triggers.sync(&profiles, &entities);
        triggers.sync(&profiles, &entities);
        triggers.update(0.0, &mut assets, &mut system);
        assert_eq!(system.diagnostics.spawned, 0);
        load(
            &mut assets,
            &mut loader,
            &mut system,
            &["fireball", "ice_orb"],
        )
        .await;
        for _ in 0..3 {
            triggers.sync(&profiles, &entities);
            triggers.update(1.0 / 60.0, &mut assets, &mut system);
            system.update(1.0 / 60.0, |_| Some(Vec2::new(300.0, 250.0)));
        }
        assert_eq!(system.diagnostics.spawned, 2);
        assert_eq!(system.diagnostics.active, 2);
        assert!(
            system
                .extract()
                .sprites
                .iter()
                .any(|sprite| sprite.pos == [300.0, 250.0])
        );
        assets.ensure_loaded("fireball");
        assets.publish(&mut loader, &mut system.registry);
        assert_eq!(system.registry.get("fireball").unwrap().revision, 1);
        assert_eq!(system.registry.get("fireball_core").unwrap().revision, 1);
        triggers.sync(&profiles, &[]);
        triggers.update(0.0, &mut assets, &mut system);
        system.update(1.0 / 60.0, |_| None);
        assert_eq!(system.diagnostics.active, 0);
        assert!(triggers.active.is_empty());
    });
}

#[test]
fn resolved_requests_preserve_full_params_and_options_and_allow_overrides() {
    let profiles = VisualProfiles::parse(
        r#"[{
      "id": 99, "name": "custom", "rules": [{"trigger": "cast", "actions": [{
        "kind": "spawn", "effect": "any_effect",
        "params": {"tint": [0.2,0.3,0.4,1.0], "size": [12,24], "duration": 2,
          "velocity": {"x": 3,"y": 4}, "entity": 7, "pos": {"x": 8,"y": 9},
          "scale": 2, "direction": {"x": -1,"y": 0}, "lifetime": 3},
        "options": {"refresh": true, "looping": true,
          "event_key": {"session": 1,"event": 2}, "prediction": {"session": 1,"action": 3},
          "attachment": {"entity": 7,"offset": {"x": 1,"y": 2},"on_missing": "detach"}},
        "bindings": {"position": false,"direction": false}
      }]}]
    }]"#,
    )
    .unwrap();
    let mut triggers = VisualTriggers::default();
    triggers.emit_with(&profiles, 99, &VisualTrigger::Cast, &context(), |request| {
        request.params.scale = 5.0;
        request.options.refresh = false;
    });
    let request = &triggers.pending.front().unwrap().request;
    assert_eq!(request.params.scale, 5.0);
    assert_eq!(request.params.pos, Vec2::new(8.0, 9.0));
    assert_eq!(request.params.direction, Vec2::new(-1.0, 0.0));
    assert_eq!(request.params.velocity, Vec2::new(3.0, 4.0));
    assert_eq!(request.params.entity, Some(7));
    assert_eq!(request.params.size, Some([12.0, 24.0]));
    assert_eq!(request.params.duration, Some(2.0));
    assert_eq!(request.params.tint, [0.2, 0.3, 0.4, 1.0]);
    assert_eq!(request.params.lifetime, 3.0);
    assert!(!request.options.refresh);
    assert!(request.options.looping);
    assert_eq!(
        request.options.event_key,
        Some(VisualEventKey {
            session: 1,
            event: 2
        })
    );
    assert_eq!(
        request.options.prediction,
        Some(PredictionKey {
            session: 1,
            action: 3
        })
    );
    assert_eq!(
        request.options.attachment.unwrap().offset,
        Vec2::new(1.0, 2.0)
    );
}

#[test]
fn impact_cancels_unloaded_flight_and_plays_a_deduplicated_world_effect() {
    runtime().block_on(async {
        let profiles = VisualProfiles::bundled().unwrap();
        let mut assets = VisualAssets::bundled().unwrap();
        let mut loader = AssetLoader::new();
        let mut system = VfxSystem::default();
        let mut triggers = VisualTriggers::default();
        triggers.sync(&profiles, &[entity(42, 1)]);
        let event = VisualEvent {
            id: 900,
            profile: 1,
            trigger: VisualTrigger::Impact,
            entity: Some(42),
            position: [110.0, 220.0],
            direction: [1.0, 0.0],
        };
        triggers.event(&profiles, &event);
        triggers.event(&profiles, &event);
        assert_eq!(triggers.pending.len(), 1);
        load(&mut assets, &mut loader, &mut system, &["fireball_impact"]).await;
        triggers.sync(&profiles, &[]);
        triggers.update(0.01, &mut assets, &mut system);
        system.update(1.0 / 60.0, |_| None);
        assert_eq!(system.diagnostics.spawned, 1);
        assert!(!system.extract().sprites.is_empty());
        triggers.clear(&mut system);
        system.update(1.0 / 60.0, |_| None);
        assert_eq!(system.diagnostics.active, 0);
        assert!(triggers.pending.is_empty());
    });
}

#[test]
fn stop_cancels_queued_tickets_and_multiple_named_occurrences_are_independent() {
    let profiles = VisualProfiles::parse(
        r#"[{
      "id": 99, "name": "two_effects", "rules": [
        {"trigger": "cast", "actions": [
          {"kind": "spawn", "effect": "gameplay/entity_flash", "track_as": "a"},
          {"kind": "spawn", "effect": "gameplay/entity_flash", "track_as": "b"}
        ]},
        {"trigger": "stop", "actions": [{"kind": "stop", "tracked": "a", "mode": "immediate"}]}
      ]
    }]"#,
    )
    .unwrap();
    let mut assets = VisualAssets::bundled().unwrap();
    let mut system = VfxSystem::new(VfxBudgets {
        requests_per_frame: 1,
        ..Default::default()
    });
    system.registry.register_gameplay_effects().unwrap();
    let mut triggers = VisualTriggers::default();
    triggers.emit(&profiles, 99, &VisualTrigger::Cast, &context());
    triggers.update(0.0, &mut assets, &mut system);
    assert_eq!(triggers.active.len(), 2);
    triggers.emit(&profiles, 99, &VisualTrigger::Stop, &context());
    triggers.update(0.0, &mut assets, &mut system);
    system.update(1.0 / 60.0, |_| None);
    assert_eq!(system.diagnostics.spawned, 1);
    triggers.update(0.0, &mut assets, &mut system);
    assert_eq!(triggers.active.len(), 1);
    triggers.clear(&mut system);
    system.update(0.0, |_| None);
    assert_eq!(system.diagnostics.active, 0);
}

#[test]
fn owner_disappearance_and_expired_events_discard_waiting_requests() {
    let profiles = VisualProfiles::bundled().unwrap();
    let mut assets = VisualAssets::bundled().unwrap();
    let mut system = VfxSystem::default();
    let mut triggers = VisualTriggers::default();
    triggers.sync(&profiles, &[entity(42, 1)]);
    triggers.sync(&profiles, &[]);
    assert!(triggers.pending.is_empty());
    triggers.emit(&profiles, 1, &VisualTrigger::Impact, &context());
    triggers.update(1.0, &mut assets, &mut system);
    assert!(triggers.pending.is_empty());
    assert_eq!(system.diagnostics.spawned, 0);
}

#[test]
fn screen_effect_needs_no_entity_and_profile_changes_replace_owned_flights() {
    runtime().block_on(async {
        let profiles = VisualProfiles::bundled().unwrap();
        let mut assets = VisualAssets::bundled().unwrap();
        let mut loader = AssetLoader::new();
        let mut system = VfxSystem::default();
        let mut triggers = VisualTriggers::default();
        load(
            &mut assets,
            &mut loader,
            &mut system,
            &["gameplay/player_hit", "fireball", "ice_orb"],
        )
        .await;
        triggers.emit(
            &profiles,
            12,
            &VisualTrigger::Hit,
            &VisualContext {
                entity: None,
                ..context()
            },
        );
        triggers.sync(&profiles, &[entity(42, 1)]);
        triggers.update(0.0, &mut assets, &mut system);
        system.update(1.0 / 60.0, |_| Some(Vec2::zero()));
        assert_eq!(system.diagnostics.spawned, 2);
        assert!(!system.extract().post_process.is_empty());
        triggers.sync(&profiles, &[entity(42, 2)]);
        triggers.update(0.0, &mut assets, &mut system);
        system.update(1.0 / 60.0, |_| Some(Vec2::zero()));
        triggers.update(0.0, &mut assets, &mut system);
        assert_eq!(system.diagnostics.spawned, 3);
        assert_eq!(system.diagnostics.active, 2);
    });
}

#[test]
fn custom_detach_policy_keeps_the_effect_alive_after_its_owner_disappears() {
    runtime().block_on(async {
        let profiles = VisualProfiles::bundled().unwrap();
        let mut assets = VisualAssets::bundled().unwrap();
        let mut loader = AssetLoader::new();
        let mut system = VfxSystem::default();
        let mut triggers = VisualTriggers::default();
        load(&mut assets, &mut loader, &mut system, &["fireball"]).await;
        // Track the source, then replace its initial request using the public customization hook.
        triggers.sync(&profiles, &[entity(42, 1)]);
        triggers.emit_with(&profiles, 1, &VisualTrigger::Spawn, &context(), |request| {
            request.options.attachment.as_mut().unwrap().on_missing = MissingOwner::Detach;
        });
        triggers.update(0.0, &mut assets, &mut system);
        system.update(1.0 / 60.0, |_| Some(Vec2::new(300.0, 250.0)));
        triggers.sync(&profiles, &[]);
        for _ in 0..3 {
            triggers.update(1.0 / 60.0, &mut assets, &mut system);
            system.update(1.0 / 60.0, |_| None);
        }
        assert_eq!(system.diagnostics.spawned, 1);
        assert_eq!(system.diagnostics.active, 1);
        assert!(
            system
                .extract()
                .sprites
                .iter()
                .any(|sprite| sprite.pos == [300.0, 250.0])
        );
        triggers.clear(&mut system);
        system.update(0.0, |_| None);
        assert_eq!(system.diagnostics.active, 0);
    });
}
