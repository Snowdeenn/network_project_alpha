use super::*;

const PARTICLE: &str = include_str!("../../../assets/vfx/exemples_vfx/particle.json");
const MESH: &str = include_str!("../../../assets/vfx/exemples_vfx/mesh.json");

fn system() -> VfxSystem {
    let mut system = VfxSystem::default();
    system
        .registry
        .register_json(PARTICLE, Priority::Normal)
        .unwrap();
    system
        .registry
        .register_json(MESH, Priority::Normal)
        .unwrap();
    system
}

fn spawn(system: &mut VfxSystem) -> VfxHandle {
    let api = system.api();
    let ticket = api
        .spawn("impact_embers", VfxSpawnParams::default())
        .unwrap();
    system.update(0.0, |_| None);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!("spawn rejected");
    };
    handle
}

#[test]
fn result_is_pending_then_resolved_and_consumed() {
    let mut system = system();
    let api = system.api();
    let ticket = api
        .spawn("impact_embers", VfxSpawnParams::default())
        .unwrap();
    assert_eq!(api.take_result(ticket), None);
    system.update(0.0, |_| None);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!();
    };
    assert!(system.is_alive(handle));
    assert_eq!(api.take_result(ticket), None);
    let invalid = api.spawn("missing", VfxSpawnParams::default()).unwrap();
    system.update(0.0, |_| None);
    assert_eq!(
        api.take_result(invalid),
        Some(SpawnResult::Rejected(VfxRejection::UnknownEffect))
    );
}

#[test]
fn full_queue_and_unread_results_remain_bounded_and_cancelable() {
    let mut system = VfxSystem::new(VfxBudgets {
        queued_requests: 1,
        ..VfxBudgets::default()
    });
    let api = system.api();
    let ticket = api.spawn("missing", VfxSpawnParams::default()).unwrap();
    assert_eq!(
        api.spawn("missing", VfxSpawnParams::default()),
        Err(super::super::QueueFull)
    );
    assert!(api.cancel_pending(ticket));
    assert!(!api.cancel_pending(ticket));
    assert!(api.spawn("missing", VfxSpawnParams::default()).is_err());
    assert_eq!(
        api.take_result(ticket),
        Some(SpawnResult::Rejected(VfxRejection::Cancelled))
    );
    assert!(api.spawn("missing", VfxSpawnParams::default()).is_ok());
    system.update(0.0, |_| None);
    assert!(system.diagnostics.queue_rejected >= 2);
}

#[test]
fn stale_handle_never_stops_recycled_slot() {
    let mut system = system();
    let first = spawn(&mut system);
    assert!(system.stop(first, StopMode::Immediate));
    let second = spawn(&mut system);
    assert_eq!(first.index, second.index);
    assert_ne!(first.generation, second.generation);
    assert!(!system.stop(first, StopMode::Immediate));
    assert!(system.is_alive(second));
}

#[test]
fn drain_keeps_residual_particles_and_recycles_after_expiration() {
    let mut system = system();
    let handle = spawn(&mut system);
    system.update(STEP, |_| None);
    assert_eq!(system.extract().sprites.len(), 24);
    assert!(system.stop(handle, StopMode::Drain));
    assert!(system.is_alive(handle));
    for _ in 0..120 {
        system.update(STEP, |_| None);
    }
    assert!(!system.is_alive(handle));
    assert!(system.extract().sprites.is_empty());
}

#[test]
fn dedup_and_authoritative_confirmation_reuse_prediction() {
    let mut system = system();
    let api = system.api();
    let prediction = PredictionKey {
        session: 1,
        action: 42,
    };
    let first = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                prediction: Some(prediction),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    api.reconcile(prediction, true).unwrap();
    let event_key = VisualEventKey {
        session: 1,
        event: 17,
    };
    let confirmed = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                prediction: Some(prediction),
                event_key: Some(event_key),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    let repeated = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                event_key: Some(event_key),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    system.update(0.0, |_| None);
    let result = api.take_result(first);
    assert_eq!(api.take_result(confirmed), result);
    assert_eq!(api.take_result(repeated), result);
    assert_eq!(system.diagnostics.spawned, 1);
    api.reconcile(prediction, false).unwrap();
    system.update(0.0, |_| None);
    assert_eq!(system.running.iter().count(), 0);
}

#[test]
fn invalid_assets_keep_previous_revision_and_active_instances_keep_old_data() {
    let mut system = system();
    let handle = spawn(&mut system);
    let mut changed: serde_json::Value = serde_json::from_str(PARTICLE).unwrap();
    changed["logical"]["emission"]["count"] = 2.into();
    system
        .registry
        .register_json(&changed.to_string(), Priority::Normal)
        .unwrap();
    assert_eq!(system.registry.get("impact_embers").unwrap().revision, 2);
    let second = spawn(&mut system);
    changed["logical"]["initialization"]["lifetime_seconds"]["min"] = 5.into();
    assert!(
        system
            .registry
            .register_json(&changed.to_string(), Priority::Normal)
            .is_err()
    );
    assert_eq!(system.registry.get("impact_embers").unwrap().revision, 2);
    system.update(STEP, |_| None);
    assert_eq!(system.extract().sprites.len(), 26);
    assert!(system.is_alive(handle) && system.is_alive(second));
    system.stop(handle, StopMode::Immediate);
    system.update(0.0, |_| None);
    assert_eq!(system.registry.assets.iter().count(), 2);
}

#[test]
fn particle_budget_refuses_whole_effect_and_large_delta_is_bounded() {
    let mut system = VfxSystem::new(VfxBudgets {
        particles: 24,
        ..VfxBudgets::default()
    });
    system
        .registry
        .register_json(PARTICLE, Priority::Normal)
        .unwrap();
    let first = spawn(&mut system);
    let api = system.api();
    let ticket = api
        .spawn("impact_embers", VfxSpawnParams::default())
        .unwrap();
    system.update(10.0, |_| None);
    assert_eq!(
        api.take_result(ticket),
        Some(SpawnResult::Rejected(VfxRejection::ElementBudget))
    );
    assert!(system.is_alive(first));
    assert!(system.diagnostics.discarded_seconds > 9.0);
}

#[test]
fn attachment_follows_then_owner_loss_cancels_all_components() {
    let mut system = system();
    let api = system.api();
    let ticket = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                attachment: Some(Attachment {
                    entity: 7,
                    offset: Vec2::new(3.0, 0.0),
                    on_missing: MissingOwner::Immediate,
                }),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    system.update(STEP, |id| (id == 7).then_some(Vec2::new(100.0, 0.0)));
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!();
    };
    let effect = system.running.get(handle).unwrap();
    assert_eq!(
        system
            .instances
            .get(effect.components[0])
            .unwrap()
            .params
            .pos
            .x,
        103.0
    );
    system.update(STEP, |_| None);
    assert!(!system.is_alive(handle));
    assert!(system.extract().sprites.is_empty());
}

#[test]
fn delayed_composite_uses_existing_dag_and_rejects_cycles() {
    let mut system = system();
    system
        .registry
        .register_composite(
            "combo",
            &[("impact_embers", 0.0), ("impact_embers", 0.05)],
            Priority::Normal,
        )
        .unwrap();
    assert!(
        system
            .registry
            .register_composite("impact_embers", &[("combo", 0.0)], Priority::Normal)
            .is_err()
    );
    let api = system.api();
    let ticket = api.spawn("combo", VfxSpawnParams::default()).unwrap();
    system.update(STEP, |_| None);
    assert!(matches!(
        api.take_result(ticket),
        Some(SpawnResult::Spawned(_))
    ));
    assert_eq!(system.extract().sprites.len(), 24);
    system.update(0.1, |_| None);
    assert_eq!(system.extract().sprites.len(), 48);
}

#[test]
fn json_composite_loads_after_its_children_and_keeps_them_until_end() {
    let source = include_str!("../../../assets/vfx/exemples_vfx/composite.json");
    let mut empty = VfxRegistry::default();
    assert!(empty.register_json(source, Priority::Normal).is_err());
    let mut system = system();
    system
        .registry
        .register_json(source, Priority::Normal)
        .unwrap();
    let api = system.api();
    let ticket = api
        .spawn("impact_composite", VfxSpawnParams::default())
        .unwrap();
    system.update(0.0, |_| None);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!();
    };
    let mut changed: serde_json::Value = serde_json::from_str(PARTICLE).unwrap();
    changed["logical"]["emission"]["count"] = 2.into();
    system
        .registry
        .register_json(&changed.to_string(), Priority::Normal)
        .unwrap();
    system.update(0.1, |_| None);
    assert_eq!(system.extract().sprites.len(), 25);
    system.stop(handle, StopMode::Immediate);
    system.update(0.0, |_| None);
    // La révision publiée du composite conserve les révisions de ses enfants.
    assert_eq!(system.registry.assets.iter().count(), 3);
}

#[test]
fn looping_keeps_handle_and_drain_cancels_delayed_components_and_restart() {
    let mut system = system();
    system
        .registry
        .register_composite(
            "loop",
            &[("impact_embers", 0.0), ("impact_embers", 0.5)],
            Priority::Normal,
        )
        .unwrap();
    let api = system.api();
    let ticket = api
        .spawn_with(
            "loop",
            VfxSpawnParams::default(),
            SpawnOptions {
                looping: true,
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    system.update(STEP, |_| None);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!();
    };
    for _ in 0..240 {
        system.update(STEP, |_| None);
    }
    assert!(system.is_alive(handle));
    let command = api.stop(handle, StopMode::Drain).unwrap();
    system.update(STEP, |_| None);
    assert_eq!(api.take_command_result(command), Some(Ok(())));
    for _ in 0..180 {
        system.update(STEP, |_| None);
    }
    assert!(!system.is_alive(handle));
    assert!(system.extract().sprites.is_empty());
}

#[test]
fn drain_before_delayed_start_does_not_activate_child_later() {
    let mut system = system();
    system
        .registry
        .register_composite("late", &[("impact_embers", 0.5)], Priority::Normal)
        .unwrap();
    let api = system.api();
    let ticket = api.spawn("late", VfxSpawnParams::default()).unwrap();
    system.update(0.0, |_| None);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!();
    };
    system.stop(handle, StopMode::Drain);
    for _ in 0..60 {
        system.update(STEP, |_| None);
    }
    assert!(system.extract().sprites.is_empty());
    assert!(!system.is_alive(handle));
}

#[test]
fn seeded_simulation_matches_across_frame_partitions_and_snapshot_is_owned() {
    let mut first = VfxSystem::with_seed(VfxBudgets::default(), 17);
    let mut second = VfxSystem::with_seed(VfxBudgets::default(), 17);
    for system in [&mut first, &mut second] {
        system
            .registry
            .register_json(PARTICLE, Priority::Normal)
            .unwrap();
        spawn(system);
    }
    first.update(STEP * 4.0, |_| None);
    for _ in 0..4 {
        second.update(STEP, |_| None);
    }
    let snapshot = first.extract();
    assert_eq!(snapshot.sprites, second.extract().sprites);
    first.update(STEP, |_| None);
    assert_ne!(snapshot.sprites, first.extract().sprites);
    assert_eq!(snapshot.sprites, second.extract().sprites);
}

#[test]
fn asynchronous_commands_report_stale_handles_and_invalid_changes() {
    let mut system = system();
    let old = spawn(&mut system);
    system.stop(old, StopMode::Immediate);
    let current = spawn(&mut system);
    let api = system.api();
    let stale = api.stop(old, StopMode::Immediate).unwrap();
    let invalid = api.update(current, VfxChange::Scale(f32::NAN)).unwrap();
    let change = api
        .update(current, VfxChange::Position(Vec2::new(19.0, 4.0)))
        .unwrap();
    assert_eq!(api.take_command_result(stale), None);
    system.update(0.0, |_| None);
    assert_eq!(
        api.take_command_result(stale),
        Some(Err(VfxCommandError::StaleHandle))
    );
    assert_eq!(
        api.take_command_result(invalid),
        Some(Err(VfxCommandError::InvalidParams))
    );
    assert_eq!(api.take_command_result(change), Some(Ok(())));
    assert!(system.is_alive(current));
}

#[test]
fn session_keys_and_request_budget_preserve_independent_occurrences() {
    let mut system = VfxSystem::new(VfxBudgets {
        requests_per_frame: 1,
        ..VfxBudgets::default()
    });
    system
        .registry
        .register_json(PARTICLE, Priority::Normal)
        .unwrap();
    let api = system.api();
    let first = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                event_key: Some(VisualEventKey {
                    session: 1,
                    event: 42,
                }),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    let second = api
        .spawn_with(
            "impact_embers",
            VfxSpawnParams::default(),
            SpawnOptions {
                event_key: Some(VisualEventKey {
                    session: 2,
                    event: 42,
                }),
                ..SpawnOptions::default()
            },
        )
        .unwrap();
    system.update(0.0, |_| None);
    assert!(api.take_result(first).is_some());
    assert_eq!(api.take_result(second), None);
    system.update(0.0, |_| None);
    assert!(api.take_result(second).is_some());
    assert_eq!(system.running.iter().count(), 2);
}

#[test]
fn every_existing_family_loads_and_invalid_numbers_are_rejected() {
    let mut registry = VfxRegistry::default();
    for source in [
        PARTICLE,
        MESH,
        include_str!("../../../assets/vfx/exemples_vfx/trail.json"),
        include_str!("../../../assets/vfx/exemples_vfx/decal.json"),
        include_str!("../../../assets/vfx/exemples_vfx/light.json"),
        include_str!("../../../assets/vfx/exemples_vfx/post-process.json"),
    ] {
        registry.register_json(source, Priority::Normal).unwrap();
    }
    let mut invalid: serde_json::Value = serde_json::from_str(MESH).unwrap();
    invalid["logical"]["emission"]["delay_seconds"] = (-1).into();
    assert!(
        registry
            .register_json(&invalid.to_string(), Priority::Normal)
            .is_err()
    );
    assert!(
        registry
            .register_json(
                &MESH.replace("\"version\": 1", "\"version\": 99"),
                Priority::Normal
            )
            .is_err()
    );
    assert!(
        registry
            .register_json(
                &MESH.replace("\"version\": 1", "\"version\": 1e100"),
                Priority::Normal
            )
            .is_err()
    );
}
