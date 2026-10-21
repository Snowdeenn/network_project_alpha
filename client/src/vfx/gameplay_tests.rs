use super::*;
use crate::vfx::{SpawnResult, StopMode, VfxBudgets, VfxRejection, VfxSystem};

const STEP: f32 = 1.0 / 60.0;
fn system() -> VfxSystem {
    let mut system = VfxSystem::with_seed(VfxBudgets::default(), 42);
    system.registry.register_gameplay_effects().unwrap();
    system
}
fn advance(system: &mut VfxSystem, steps: usize) {
    for _ in 0..steps {
        system.update(STEP, |_| Some(Vec2::zero()));
    }
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn migrated_effects_share_seeded_instances_particles_and_render_extraction() {
    let mut system = system();
    let api = system.api();
    api.movement_dust(Vec2::new(100.0, 100.0), Vec2::new(2.0, 1.0))
        .unwrap();
    api.entity_hit(Vec2::new(50.0, 50.0)).unwrap();
    api.slash(Vec2::new(10.0, 20.0), Vec2::new(0.0, 1.0), 20.0, 30.0)
        .unwrap();
    api.dash_ghost(Vec2::new(5.0, 6.0), 0.2, Color::BLUE)
        .unwrap();
    advance(&mut system, 1);
    let snapshot = system.extract();
    assert_eq!(snapshot.sprites.len(), 12);
    assert_eq!(snapshot.rings.len(), 1);
    let ring = &snapshot.rings[0];
    assert_eq!(ring.radii, [10.0, 30.0]);
    close(ring.rotation_degrees, 90.0);
    close(ring.color[3], 1.0 - STEP / 0.15);
    let ghost = snapshot
        .sprites
        .iter()
        .find(|sprite| sprite.pos == [25.0, 26.0])
        .unwrap();
    assert_eq!(ghost.size, [40.0; 2]);
    close(ghost.color[2], 1.0);
    close(ghost.color[3], 0.5 * (1.0 - STEP / 0.2));
    let dust = snapshot
        .sprites
        .iter()
        .find(|sprite| sprite.pos[1] == 120.0)
        .unwrap();
    close(dust.size[0], 1.6);
    advance(&mut system, 1);
    let grown = system.extract();
    assert!(
        grown
            .sprites
            .iter()
            .any(|sprite| sprite.pos[1] > 110.0 && sprite.size[0] > dust.size[0])
    );
    close(system.take_camera_trauma(), 0.5);
    close(system.take_camera_trauma(), 0.0);
    advance(&mut system, 60);
    assert_eq!(system.diagnostics.active, 0);
    assert!(system.extract().sprites.is_empty());
    assert!(system.extract().rings.is_empty());
}

#[test]
fn detached_gameplay_events_do_not_fill_unread_result_storage() {
    let mut system = VfxSystem::with_seed(
        VfxBudgets {
            queued_requests: 4,
            ..Default::default()
        },
        42,
    );
    system.registry.register_gameplay_effects().unwrap();
    let api = system.api();
    for _ in 0..100 {
        api.movement_dust(Vec2::zero(), Vec2::new(1.0, 0.0))
            .unwrap();
        advance(&mut system, 1);
    }
    assert!(api.requests.borrow().results.is_empty());
    assert_eq!(system.diagnostics.spawned, 100);
    assert_eq!(system.diagnostics.rejected, 0);
}

#[test]
fn composite_impact_is_rejected_atomically_by_shared_particle_budget() {
    let mut system = VfxSystem::with_seed(
        VfxBudgets {
            particles: 9,
            ..Default::default()
        },
        42,
    );
    system.registry.register_gameplay_effects().unwrap();
    let api = system.api();
    let ticket = api.spawn(ENTITY_HIT, VfxSpawnParams::default()).unwrap();
    advance(&mut system, 1);
    assert_eq!(
        api.take_result(ticket),
        Some(SpawnResult::Rejected(VfxRejection::ElementBudget))
    );
    assert_eq!(system.diagnostics.active, 0);
    assert!(system.extract().sprites.is_empty());
    assert_eq!(system.take_camera_trauma(), 0.0);
}

#[test]
fn manual_sword_trail_keeps_latest_points_refreshes_and_cleans_missing_owners() {
    let mut system = system();
    let api = system.api();
    for index in 0..20 {
        api.trail_point(7, Vec2::new(index as f32, 0.0), Color::RED)
            .unwrap();
    }
    advance(&mut system, 1);
    let snapshot = system.extract();
    assert_eq!(snapshot.ribbons.len(), 1);
    let points = &snapshot.ribbons[0].points;
    assert_eq!(points.len(), 12);
    assert_eq!(points[0].0, [8.0, 0.0]);
    assert_eq!(points[11].0, [19.0, 0.0]);
    assert_eq!(points[11].2, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(system.diagnostics.spawned, 1);
    advance(&mut system, 2);
    api.trail_point(7, Vec2::new(30.0, 0.0), Color::RED)
        .unwrap();
    advance(&mut system, 1);
    let refreshed = system.extract();
    let points = &refreshed.ribbons[0].points;
    assert_eq!(points.last().unwrap().0, [30.0, 0.0]);
    assert!(points[0].1 < points.last().unwrap().1);
    assert!(points[0].2[3] < points.last().unwrap().2[3]);
    for _ in 0..12 {
        system.update(STEP, |_| None);
    }
    assert_eq!(system.diagnostics.active, 0);
    assert!(system.extract().ribbons.is_empty());
}

#[test]
fn entity_flash_refreshes_without_duplicates_and_expires() {
    let mut system = system();
    let api = system.api();
    api.entity_flash(7, 0.1).unwrap();
    advance(&mut system, 3);
    assert_eq!(system.entity_tint(7), Some([1.0; 4]));
    assert_eq!(system.entity_tint(8), None);
    api.entity_flash(7, 0.2).unwrap();
    advance(&mut system, 8);
    assert_eq!(system.diagnostics.active, 1);
    assert!(system.entity_tint(7).is_some());
    advance(&mut system, 6);
    assert!(system.entity_tint(7).is_none());
    assert_eq!(system.diagnostics.active, 0);
}

#[test]
fn blind_fades_in_holds_fades_out_and_can_be_replaced_or_stopped() {
    let mut system = system();
    let api = system.api();
    api.blind(3.0).unwrap();
    advance(&mut system, 30);
    close(system.post_process("blind").weight, 0.5);
    advance(&mut system, 60);
    close(system.post_process("blind").weight, 1.0);
    advance(&mut system, 60);
    close(system.post_process("blind").weight, 0.5);
    advance(&mut system, 31);
    close(system.post_process("blind").weight, 0.0);
    api.blind(0.5).unwrap();
    advance(&mut system, 15);
    close(system.post_process("blind").weight, 0.25);
    api.blind(3.0).unwrap();
    advance(&mut system, 1);
    assert_eq!(system.diagnostics.active, 1);
    close(system.post_process("blind").remaining_seconds, 3.0 - STEP);
    api.stop_named(BLIND).unwrap();
    advance(&mut system, 1);
    close(system.post_process("blind").weight, 0.0);
    api.blind(0.0).unwrap();
    advance(&mut system, 1);
    assert_eq!(system.diagnostics.active, 0);
}

#[test]
fn screen_flash_uses_instance_duration_and_immediate_stop_releases_visuals() {
    let mut system = system();
    let api = system.api();
    api.player_hit().unwrap();
    advance(&mut system, 6);
    close(system.post_process("hit_flash").weight, 0.5);
    api.screen_flash(2.0).unwrap();
    advance(&mut system, 60);
    close(system.post_process("hit_flash").weight, 0.5);
    api.stop_named(PLAYER_HIT).unwrap();
    advance(&mut system, 1);
    close(system.post_process("hit_flash").weight, 0.0);
    let ticket = api.spawn(DASH_GHOST, VfxSpawnParams::default()).unwrap();
    advance(&mut system, 1);
    let Some(SpawnResult::Spawned(handle)) = api.take_result(ticket) else {
        panic!()
    };
    assert!(system.stop(handle, StopMode::Immediate));
    assert!(system.extract().sprites.is_empty());
}

#[test]
fn invalid_gameplay_inputs_are_rejected_and_seeded_dust_is_repeatable() {
    let mut a = system();
    let mut b = system();
    for system in [&mut a, &mut b] {
        system.api().entity_hit(Vec2::new(10.0, 20.0)).unwrap();
        advance(system, 5);
    }
    assert_eq!(a.extract().sprites, b.extract().sprites);
    let api = a.api();
    api.dash_ghost(Vec2::zero(), f32::NAN, Color::WHITE)
        .unwrap();
    api.camera_shake(-1.0).unwrap();
    api.slash(Vec2::zero(), Vec2::new(1.0, 0.0), -2.0, 3.0)
        .unwrap();
    advance(&mut a, 1);
    assert_eq!(a.diagnostics.rejected, 3);
}

#[test]
fn malformed_gameplay_geometry_and_feedback_keep_the_last_valid_revision() {
    let mut system = system();
    let cases = [
        (
            include_str!("../../../assets/vfx/gameplay/movement_dust.json"),
            "/logical/initialization/position/min",
            serde_json::json!([40.0, 20.0]),
        ),
        (
            include_str!("../../../assets/vfx/gameplay/slash.json"),
            "/visual/representations/0/resolution",
            serde_json::json!(0),
        ),
        (
            include_str!("../../../assets/vfx/gameplay/camera_shake.json"),
            "/feedback/trauma",
            serde_json::json!(-1.0),
        ),
    ];
    for (source, path, invalid) in cases {
        let mut document: serde_json::Value = serde_json::from_str(source).unwrap();
        let name = document["name"].as_str().unwrap().to_owned();
        let previous = system.registry.get(&name).unwrap().revision;
        *document.pointer_mut(path).unwrap() = invalid;
        assert!(
            system
                .registry
                .register_json(&document.to_string(), Priority::Normal)
                .is_err(),
            "{path}"
        );
        assert_eq!(system.registry.get(&name).unwrap().revision, previous);
    }
}

#[test]
fn refreshed_manual_trails_preserve_handles_and_deduplicate_event_keys() {
    let mut system = system();
    let api = system.api();
    let first = api
        .spawn_with(
            SWORD_TRAIL,
            VfxSpawnParams {
                entity: Some(7),
                lifetime: 0.12,
                ..Default::default()
            },
            SpawnOptions {
                refresh: true,
                ..Default::default()
            },
        )
        .unwrap();
    advance(&mut system, 1);
    let first = api.take_result(first).unwrap();
    let mut tickets = Vec::new();
    for _ in 0..2 {
        tickets.push(
            api.spawn_with(
                SWORD_TRAIL,
                VfxSpawnParams {
                    pos: Vec2::new(10.0, 0.0),
                    entity: Some(7),
                    lifetime: 0.12,
                    ..Default::default()
                },
                SpawnOptions {
                    refresh: true,
                    event_key: Some(crate::vfx::VisualEventKey {
                        session: 1,
                        event: 2,
                    }),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
    }
    advance(&mut system, 1);
    for ticket in tickets {
        assert_eq!(api.take_result(ticket), Some(first));
    }
    assert_eq!(system.extract().ribbons[0].points.len(), 2);
    assert_eq!(system.diagnostics.duplicates, 1);
}
