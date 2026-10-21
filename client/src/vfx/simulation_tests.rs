use super::*;
use crate::vfx::instance::{VfxInstanceId, VfxInstanceManager};
use crate::vfx::storage::AssetRegister;

const MESH: &str = include_str!("../../../assets/config/vfx/exemples_vfx/mesh.json");
const DECAL: &str = include_str!("../../../assets/config/vfx/exemples_vfx/decal.json");
const LIGHT: &str = include_str!("../../../assets/config/vfx/exemples_vfx/light.json");
const TRAIL: &str = include_str!("../../../assets/config/vfx/exemples_vfx/trail.json");
const POST_PROCESS: &str =
    include_str!("../../../assets/config/vfx/exemples_vfx/post-process.json");

fn effect(source: &str) -> (AssetRegister, VfxInstanceManager, VfxInstanceId) {
    let description = crate::vfx::parser::parse_effect(source).unwrap();
    let mut descriptions = AssetRegister::new();
    let descriptor = descriptions.insert(description);
    let mut manager = VfxInstanceManager::new();
    let id = manager.create_instance(descriptor, params(), Vec::new());
    manager.get_mut(id).unwrap().state = VfxInstanceState::Active;
    (descriptions, manager, id)
}

fn params() -> VfxSpawnParams {
    VfxSpawnParams {
        pos: Vec2::zero(),
        scale: 1.0,
        direction: Vec2::new(1.0, 0.0),
        lifetime: 0.5,
    }
}

fn simulation(manager: &VfxInstanceManager, id: VfxInstanceId) -> &VfxSimulation {
    manager.get(id).unwrap().simulation.as_ref().unwrap()
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn waiting_does_not_initialize_or_age_any_family() {
    for source in [MESH, DECAL, LIGHT, TRAIL, POST_PROCESS] {
        let (descriptions, mut manager, id) = effect(source);
        manager.get_mut(id).unwrap().state = VfxInstanceState::Waiting;
        manager.simulate(10.0, &descriptions);
        let instance = manager.get(id).unwrap();
        assert_eq!(instance.age, 0.0);
        assert!(instance.simulation.is_none());
    }
}

#[test]
fn mesh_expands_through_chained_transitions_then_fades() {
    let (descriptions, mut manager, id) = effect(MESH);
    manager.get_mut(id).unwrap().params.scale = 2.0;
    manager.simulate(0.125, &descriptions);
    let VfxSimulation::Mesh(elements) = simulation(&manager, id) else {
        panic!()
    };
    assert_eq!(elements.len(), 1);
    close(elements[0].size[0], 120.0);
    manager.simulate(0.25, &descriptions);
    let VfxSimulation::Mesh(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].size[0], 208.0);
    manager.simulate(0.175, &descriptions);
    let VfxSimulation::Mesh(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].color[3], 0.1);
    close(elements[0].size[0], 235.2);
    manager.simulate(0.1, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn decal_uses_descriptor_lifetime_and_fades_from_current_alpha() {
    let (descriptions, mut manager, id) = effect(DECAL);
    manager.simulate(2.5, &descriptions);
    let VfxSimulation::Decal(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].color[3], 0.6);
    close(elements[0].rotation_degrees, 12.0);
    manager.simulate(0.7, &descriptions);
    let VfxSimulation::Decal(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].color[3], 0.15);
    manager.simulate(0.3, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn light_updates_color_radius_intensity_then_extinguishes() {
    let (descriptions, mut manager, id) = effect(LIGHT);
    manager.simulate(0.09, &descriptions);
    let VfxSimulation::Light(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].intensity, 2.15);
    close(elements[0].radius, 72.0);
    close(elements[0].color[1], 0.45);
    manager.simulate(0.15, &descriptions);
    let VfxSimulation::Light(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].intensity, 0.15);
    manager.simulate(0.1, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn post_process_interpolates_scalar_vector_and_weight() {
    let (descriptions, mut manager, id) = effect(POST_PROCESS);
    manager.simulate(0.1525, &descriptions);
    let VfxSimulation::PostProcess(effect) = simulation(&manager, id) else {
        panic!()
    };
    let PostProcessValue::Scalar(bloom) = effect.parameters["bloom_strength"] else {
        panic!()
    };
    close(bloom, 0.4);
    let PostProcessValue::Vector(offset) = &effect.parameters["chromatic_offset_pixels"] else {
        panic!()
    };
    close(offset[0], 1.0);
    manager.simulate(0.2475, &descriptions);
    let VfxSimulation::PostProcess(effect) = simulation(&manager, id) else {
        panic!()
    };
    close(effect.weight, 0.5);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finishing);
    manager.simulate(0.1, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn continuous_meshes_keep_same_count_age_and_size_across_frame_partitions() {
    let mut source: serde_json::Value = serde_json::from_str(MESH).unwrap();
    source["logical"]["emission"] =
        serde_json::json!({"mode":"continuous", "count":4, "delay_seconds":0.25});
    let source = source.to_string();
    let (descriptions, mut coarse, first) = effect(&source);
    let (_, mut fine, second) = effect(&source);
    coarse.simulate(0.875, &descriptions);
    for _ in 0..7 {
        fine.simulate(0.125, &descriptions);
    }
    let VfxSimulation::Mesh(a) = simulation(&coarse, first) else {
        panic!()
    };
    let VfxSimulation::Mesh(b) = simulation(&fine, second) else {
        panic!()
    };
    assert_eq!(a.len(), 2);
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        close(a.age, b.age);
        close(a.size[0], b.size[0]);
        close(a.color[3], b.color[3]);
    }
    coarse.simulate(1.0, &descriptions);
    assert_eq!(coarse.get(first).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn delayed_burst_accounts_for_time_after_deadline() {
    let mut source: serde_json::Value = serde_json::from_str(MESH).unwrap();
    source["logical"]["emission"]["delay_seconds"] = serde_json::json!(0.5);
    let (descriptions, mut manager, id) = effect(&source.to_string());
    manager.simulate(0.25, &descriptions);
    let VfxSimulation::Mesh(elements) = simulation(&manager, id) else {
        panic!()
    };
    assert!(elements.is_empty());
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Active);
    manager.simulate(0.375, &descriptions);
    let VfxSimulation::Mesh(elements) = simulation(&manager, id) else {
        panic!()
    };
    close(elements[0].age, 0.125);
    close(elements[0].size[0], 60.0);
}

fn trail_source() -> serde_json::Value {
    let mut source: serde_json::Value = serde_json::from_str(TRAIL).unwrap();
    source["logical"]["emission"]["rate_per_second"] = serde_json::json!(4.0);
    source["logical"]["emission"]["duration_seconds"] = serde_json::json!(0.5);
    source["logical"]["initialization"]["lifetime_seconds"] =
        serde_json::json!({"min":1.0,"max":1.0});
    source["logical"]["evolution"]["point_lifetime_seconds"] = serde_json::json!(0.5);
    source["visual"]["visual_disappearance"]["duration_seconds"] = serde_json::json!(0.25);
    source["visual"]["transitions"] = serde_json::json!([]);
    source
}

#[test]
fn world_trail_keeps_old_positions_and_waits_for_last_point_fade() {
    let (descriptions, mut manager, id) = effect(&trail_source().to_string());
    manager.simulate(0.25, &descriptions);
    manager.get_mut(id).unwrap().params.pos = Vec2::new(10.0, 0.0);
    manager.simulate(0.25, &descriptions);
    let VfxSimulation::Trail(trail) = simulation(&manager, id) else {
        panic!()
    };
    assert_eq!(trail.points.len(), 2);
    close(trail.points[0].pos.x, 0.0);
    close(trail.points[1].pos.x, 10.0);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finishing);
    manager.simulate(0.625, &descriptions);
    let VfxSimulation::Trail(trail) = simulation(&manager, id) else {
        panic!()
    };
    assert_eq!(trail.points.len(), 1);
    close(trail.points[0].color[3], 0.5);
    manager.simulate(0.125, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn trail_retains_only_newest_points_and_handles_zero_capacity() {
    let mut source = trail_source();
    source["logical"]["evolution"]["max_points"] = serde_json::json!(1);
    let (descriptions, mut manager, id) = effect(&source.to_string());
    manager.simulate(0.5, &descriptions);
    let VfxSimulation::Trail(trail) = simulation(&manager, id) else {
        panic!()
    };
    assert_eq!(trail.points.len(), 1);
    close(trail.points[0].age, 0.0);
    source["logical"]["evolution"]["max_points"] = serde_json::json!(0);
    let (descriptions, mut manager, id) = effect(&source.to_string());
    manager.simulate(0.5, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn attachment_follows_supplied_position_and_instances_remain_independent() {
    let mut source: serde_json::Value = serde_json::from_str(LIGHT).unwrap();
    source["logical"]["initialization"]["position"] =
        serde_json::json!({"shape":"attachment", "source":"weapon_tip"});
    let (descriptions, mut manager, first) = effect(&source.to_string());
    let second =
        manager.create_instance(manager.get(first).unwrap().descriptor, params(), Vec::new());
    manager.simulate(0.1, &descriptions);
    manager.get_mut(first).unwrap().params.pos = Vec2::new(8.0, 9.0);
    manager.get_mut(second).unwrap().state = VfxInstanceState::Active;
    manager.simulate(0.0, &descriptions);
    let VfxSimulation::Light(elements) = simulation(&manager, first) else {
        panic!()
    };
    close(elements[0].pos.x, 8.0);
    close(elements[0].pos.y, 9.0);
    let VfxSimulation::Light(elements) = simulation(&manager, second) else {
        panic!()
    };
    close(elements[0].pos.x, 0.0);
    close(elements[0].age, 0.0);
}

#[test]
fn zero_lifetime_and_disappearance_release_immediately() {
    let mut source: serde_json::Value = serde_json::from_str(MESH).unwrap();
    source["logical"]["initialization"]["lifetime_seconds"] =
        serde_json::json!({"min":0.0,"max":0.0});
    source["visual"]["visual_disappearance"]["duration_seconds"] = serde_json::json!(0.0);
    let (descriptions, mut manager, id) = effect(&source.to_string());
    manager.simulate(0.0, &descriptions);
    assert_eq!(manager.get(id).unwrap().state, VfxInstanceState::Finished);
}

#[test]
fn simulation_completion_unlocks_next_dag_node() {
    use crate::vfx::dag::{VfxEdge, VfxEvent, VfxGraph, VfxNodeEffect};
    let (descriptions, mut manager, first) = effect(LIGHT);
    manager.get_mut(first).unwrap().state = VfxInstanceState::Waiting;
    let second =
        manager.create_instance(manager.get(first).unwrap().descriptor, params(), Vec::new());
    let mut graph = VfxGraph::new("sequence".into());
    let source = graph.add_node(VfxNodeEffect::Effect(first));
    let target = graph.add_node(VfxNodeEffect::Effect(second));
    graph
        .add_edge(VfxEdge {
            source,
            target,
            event: VfxEvent::Finished,
            delay_seconds: 0.0,
        })
        .unwrap();
    manager.run(&mut graph, 0.0);
    manager.simulate(1.0, &descriptions);
    manager.run(&mut graph, 1.0);
    assert_eq!(
        manager.get(first).unwrap().state,
        VfxInstanceState::Finished
    );
    assert_eq!(manager.get(second).unwrap().state, VfxInstanceState::Active);
}
