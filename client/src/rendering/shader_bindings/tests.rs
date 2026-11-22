use super::*;
use std::collections::BTreeSet;

fn verify(schema: BindingSchema, shaders: &[(&str, wgpu::ShaderStages)]) {
    let schema = schema.normalized().unwrap();
    let mut declared = BTreeSet::new();
    for &(source, stage) in shaders {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let mut layouter = naga::proc::Layouter::default();
        layouter.update(module.to_ctx()).unwrap();
        for (_, variable) in module.global_variables.iter() {
            let Some(location) = &variable.binding else {
                continue;
            };
            declared.insert((location.group, location.binding));
            let entry = schema
                .groups
                .iter()
                .find(|g| g.index == location.group)
                .and_then(|g| g.entries.iter().find(|e| e.binding == location.binding))
                .expect("WGSL resource missing from schema");
            assert!(
                entry.visibility.contains(stage),
                "resource not visible to shader stage"
            );
            assert!(entry.count.is_none());
            match (&module.types[variable.ty].inner, variable.space, entry.ty) {
                (
                    _,
                    naga::AddressSpace::Uniform,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        min_binding_size,
                        has_dynamic_offset: false,
                    },
                ) => assert_eq!(
                    min_binding_size.unwrap().get(),
                    layouter[variable.ty].size as u64
                ),
                (
                    naga::TypeInner::Image {
                        dim: naga::ImageDimension::D2,
                        arrayed: false,
                        class:
                            naga::ImageClass::Sampled {
                                kind: naga::ScalarKind::Float,
                                multi: false,
                            },
                    },
                    _,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ) => {}
                (
                    naga::TypeInner::Sampler { comparison: false },
                    _,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ) => {}
                other => panic!("WGSL/schema type mismatch: {other:?}"),
            }
        }
    }
    let provided: BTreeSet<_> = schema
        .groups
        .iter()
        .flat_map(|g| g.entries.iter().map(move |e| (g.index, e.binding)))
        .collect();
    assert_eq!(
        declared, provided,
        "schema declares resources absent from WGSL"
    );
}

const VERTEX: &str = include_str!("../../graphic_data/shader/default_textured.vert.wgsl");
const FULLSCREEN: &str = include_str!("../../graphic_data/shader/default_post_process.vert.wgsl");

#[test]
fn bundled_mesh_schemas_match_wgsl_locations_types_sizes_and_visibility() {
    for vertex in [
        VERTEX,
        include_str!("../../graphic_data/shader/default.vert.wgsl"),
    ] {
        verify(
            SHAPE_BINDINGS.schema().unwrap(),
            &[
                (vertex, wgpu::ShaderStages::VERTEX),
                (
                    include_str!("../../graphic_data/shader/default.frag.wgsl"),
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ],
        );
    }
    for fragment in [
        include_str!("../../graphic_data/shader/default_textured.frag.wgsl"),
        include_str!("../../graphic_data/shader/entity_flash.frag.wgsl"),
    ] {
        verify(
            SPRITE_BINDINGS.schema().unwrap(),
            &[
                (VERTEX, wgpu::ShaderStages::VERTEX),
                (fragment, wgpu::ShaderStages::FRAGMENT),
            ],
        );
    }
    verify(
        HEALTH_BINDINGS.schema().unwrap(),
        &[
            (VERTEX, wgpu::ShaderStages::VERTEX),
            (
                include_str!("../../graphic_data/shader/progress_bar.frag.wgsl"),
                wgpu::ShaderStages::FRAGMENT,
            ),
        ],
    );
}

#[test]
fn bundled_effect_schemas_match_wgsl() {
    for (source, size) in [
        (
            include_str!("../../graphic_data/shader/default_post_process.frag.wgsl"),
            0,
        ),
        (
            include_str!("../../graphic_data/shader/hit_flash_effect.frag.wgsl"),
            4,
        ),
        (
            include_str!("../../graphic_data/shader/blind_effect.frag.wgsl"),
            8,
        ),
    ] {
        verify(
            EffectBindings::bundled(size).schema().unwrap(),
            &[
                (FULLSCREEN, wgpu::ShaderStages::VERTEX),
                (source, wgpu::ShaderStages::FRAGMENT),
            ],
        );
    }
}

#[test]
fn contracts_allow_arbitrary_bindings_shared_groups_and_holes() {
    let layout = MeshBindings {
        camera: UniformSlot {
            location: BindingLocation::new(3, 7),
            ..SHAPE_BINDINGS.camera
        },
        parameters: Some(UniformSlot {
            location: BindingLocation::new(1, 5),
            ..HEALTH_BINDINGS.parameters.unwrap()
        }),
        texture: None,
    };
    let vertex = VERTEX.replace("@group(0) @binding(0)", "@group(3) @binding(7)");
    let fragment = include_str!("../../graphic_data/shader/progress_bar.frag.wgsl")
        .replace("@group(2) @binding(0)", "@group(1) @binding(5)");
    let schema = velum::binding_schema! {
        3 => [bindings::buffer_entry(
            7, wgpu::ShaderStages::VERTEX, wgpu::BufferBindingType::Uniform, 64, false,
        )],
        1 => [bindings::buffer_entry(
            5, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Uniform, 4, false,
        )],
    };
    assert_eq!(
        schema.normalized().unwrap(),
        layout.schema().unwrap().normalized().unwrap()
    );
    verify(
        schema,
        &[
            (&vertex, wgpu::ShaderStages::VERTEX),
            (&fragment, wgpu::ShaderStages::FRAGMENT),
        ],
    );
    let mut layout = SPRITE_BINDINGS;
    layout.texture.as_mut().unwrap().texture = BindingLocation::new(0, 4);
    layout.texture.as_mut().unwrap().sampler = BindingLocation::new(0, 8);
    assert_eq!(layout.schema().unwrap().groups.len(), 1);
}

#[test]
fn overlapping_semantic_contributions_are_rejected() {
    let mut layout = SPRITE_BINDINGS;
    layout.texture.as_mut().unwrap().texture = layout.camera.location;
    assert!(layout.schema().is_err());
    let mut layout = HEALTH_BINDINGS;
    layout.parameters.as_mut().unwrap().location = layout.camera.location;
    assert!(layout.schema().is_err());
    let mut effect = EffectBindings::bundled(4);
    effect.parameters.as_mut().unwrap().location = effect.source.sampler;
    assert!(effect.schema().is_err());
}

#[test]
fn invalid_uniform_sizes_are_rejected_before_gpu_allocation() {
    for size in [0, 3, 6] {
        let mut layout = HEALTH_BINDINGS;
        layout.parameters.as_mut().unwrap().size = size;
        assert!(layout.schema().is_err());
    }
}
