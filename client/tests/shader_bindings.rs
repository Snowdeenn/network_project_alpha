//! End-to-end bindings regression through Alpha's renderer and native Velum validation.
use client::rendering::shader_bindings::{
    self, BindingLocation as Location, EffectBindings, MeshBindings, ObjectBindings, TextureSlots,
    UniformSlot,
};
use client::rendering::{AssetCache, BlendMode, GpuContext, Primitive, SceneFrame, SceneRenderer};
use velum::backend::{
    self as gpu,
    bindings::{self, BindingResource, BindingUse},
    buffer,
    geometry::BufferRegion,
    material::{self, MaterialTemplate, MaterialTemplateId, ShaderEntry},
};
use velum::{RenderData, SharedResource};

fn pixels(gpu: &GpuContext, texture: &wgpu::Texture) -> Vec<u8> {
    let buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("bindings readback"),
        size: 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.create_encoder("bindings readback");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(2),
            },
        },
        texture.size(),
    );
    gpu.submit(encoder);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    gpu.device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    mapped
        .chunks_exact(256)
        .flat_map(|row| row[..16].to_vec())
        .collect()
}

#[test]
#[ignore = "requires a graphics adapter"]
fn alpha_renders_explicit_bindings_storage_dynamic_offsets_and_rejects_collisions() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let context =
        GpuContext::from_device(device, queue, 4, 2, wgpu::TextureFormat::Rgba8UnormSrgb).unwrap();
    let ctx = Some(context.velum_context_id());
    let mut data = RenderData::new();
    let mut assets = AssetCache::new(&context, &mut data).unwrap();
    let mut renderer = SceneRenderer::new(&context, &mut assets, &mut data).unwrap();
    let output = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("binding output"),
        size: wgpu::Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: context.surface_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let path = std::env::temp_dir().join(format!("alpha-bindings-{}.png", std::process::id()));
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))
        .save(&path)
        .unwrap();
    let texture = assets.load_texture(&context, &mut data, &path).unwrap();
    std::fs::remove_file(path).unwrap();
    let layout = MeshBindings {
        camera: UniformSlot {
            location: Location::new(2, 5),
            size: 64,
            visibility: wgpu::ShaderStages::VERTEX,
        },
        texture: Some(TextureSlots {
            texture: Location::new(2, 7),
            sampler: Location::new(3, 9),
        }),
        parameters: Some(UniformSlot {
            location: Location::new(0, 4),
            size: 16,
            visibility: wgpu::ShaderStages::FRAGMENT,
        }),
    };
    let vertex = include_str!("../src/graphic_data/shader/default_textured.vert.wgsl")
        .replace("@group(0) @binding(0)", "@group(2) @binding(5)");
    let fragment = r#"
@group(2) @binding(7) var image: texture_2d<f32>;
@group(3) @binding(9) var image_sampler: sampler;
@group(0) @binding(4) var<uniform> tint: vec4<f32>;
@group(1) @binding(6) var<storage, read> stored: vec4<f32>;
@group(1) @binding(8) var<uniform> dynamic_tint: vec4<f32>;
struct Input { @location(0) uv: vec2<f32>, @location(1) color: vec4<f32> }
@fragment fn fs_main(input: Input) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, input.uv) * tint * stored * dynamic_tint * input.color;
}
"#;
    let mut schema = layout.schema().unwrap();
    schema.groups.push(gpu::GroupSchema {
        index: 1,
        entries: vec![
            bindings::buffer_entry(
                6,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::BufferBindingType::Storage { read_only: true },
                16,
                false,
            ),
            bindings::buffer_entry(
                8,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::BufferBindingType::Uniform,
                16,
                true,
            ),
        ],
    });
    let mut template = MaterialTemplate::fullscreen(
        ctx,
        "explicit mesh",
        gpu::try_shader(ctx, "explicit vertex", &vertex).unwrap(),
        context.surface_format(),
    );
    template.fragment = Some(ShaderEntry::new(
        gpu::try_shader(ctx, "explicit fragment", fragment).unwrap(),
        "fs_main",
    ));
    template.schema = schema;
    template.vertex_layouts = vec![Some(velum::geometry::Vertex::layout().into())];
    let template = material::try_template(template).unwrap().share();
    let alignment = context
        .device()
        .limits()
        .min_uniform_buffer_offset_alignment
        .max(16);
    let dynamic = buffer::allocate_in::<u8, buffer::Uniform>(ctx, alignment as u64 + 16)
        .unwrap()
        .share();
    buffer::try_write_buffer(&dynamic.id(), 0, bytemuck::cast_slice(&[1f32, 0., 0., 1.])).unwrap();
    buffer::try_write_buffer(
        &dynamic.id(),
        alignment as u64,
        bytemuck::cast_slice(&[0f32, 1., 0., 1.]),
    )
    .unwrap();
    let stored = buffer::allocate_in::<u8, buffer::StorageRead>(ctx, 16)
        .unwrap()
        .share();
    buffer::try_write_buffer(&stored.id(), 0, bytemuck::cast_slice(&[1f32; 4])).unwrap();
    let set = velum::try_binding_set! {
        context: ctx,
        group: 1,
        entries: [
            8 => BindingResource::Buffer {
                region: BufferRegion::from_shared(dynamic).unwrap(),
                size: 16,
            },
            6 => BindingResource::buffer(BufferRegion::from_shared(stored).unwrap()),
        ],
    }
    .unwrap()
    .share();
    let usage = BindingUse::from(set.id()).with_offsets(vec![alignment]);
    data.insert([set]);

    let scene = |resources: Vec<BindingUse>, template: SharedResource<MaterialTemplateId>| {
        let mut scene = SceneFrame::new();
        scene.push_hud(Primitive::Material {
            template,
            layout,
            resource_bindings: ObjectBindings(resources),
            texture_id: Some(texture),
            pos: [0.; 2],
            size: [4., 2.],
            rotation: 0.,
            uv: None,
            tint: [1.; 4],
            uniform_data: bytemuck::cast_slice(&[1f32; 4]).to_vec(),
            blend: BlendMode::Alpha,
            layer: 0,
        });
        scene
    };
    renderer
        .render_to_texture(
            &context,
            &mut assets,
            &mut data,
            scene(vec![usage.clone()], template.clone()),
            &output,
        )
        .unwrap();
    for pixel in pixels(&context, &output).chunks_exact(4) {
        assert_eq!(pixel, [0, 255, 0, 255]);
    }
    for offsets in [
        vec![],
        vec![1],
        vec![alignment * 2],
        vec![alignment, alignment],
    ] {
        let invalid = BindingUse {
            dynamic_offsets: offsets,
            ..usage.clone()
        };
        assert!(
            renderer
                .render_to_texture(
                    &context,
                    &mut assets,
                    &mut data,
                    scene(vec![invalid], template.clone()),
                    &output
                )
                .is_err()
        );
    }
    let duplicate = renderer
        .render_to_texture(
            &context,
            &mut assets,
            &mut data,
            scene(vec![usage.clone(), usage.clone()], template.clone()),
            &output,
        )
        .unwrap_err();
    assert!(duplicate.to_string().contains("conflicting"), "{duplicate}");

    // Overlapping resource contributions fail before any partial registration.
    let view = output.create_view(&Default::default());
    assert!(
        shader_bindings::register_sets(
            ctx,
            [
                (Location::new(0, 4), BindingResource::Texture(view.clone())),
                (Location::new(0, 4), BindingResource::Texture(view)),
            ]
        )
        .is_err()
    );

    // Fullscreen layouts also use explicit, non-consecutive groups and arbitrary bindings.
    let effect_layout = EffectBindings {
        source: TextureSlots {
            texture: Location::new(3, 7),
            sampler: Location::new(3, 9),
        },
        parameters: Some(UniformSlot {
            location: Location::new(1, 5),
            size: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
        }),
    };
    let effect_source = r#"
@group(3) @binding(7) var image: texture_2d<f32>;
@group(3) @binding(9) var image_sampler: sampler;
@group(1) @binding(5) var<uniform> amount: f32;
@fragment fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(image, image_sampler, uv);
    return vec4(mix(color.rgb, vec3(1.) - color.rgb, amount), color.a);
}
"#;
    let mut wrong_layout = effect_layout;
    wrong_layout.source.texture = Location::new(2, 7);
    assert!(
        renderer
            .register_post_effect_with_bindings(
                &mut assets,
                &mut data,
                context.surface_format(),
                "mismatched WGSL",
                effect_source,
                wrong_layout
            )
            .is_err()
    );
    let effect = renderer
        .register_post_effect_with_bindings(
            &mut assets,
            &mut data,
            context.surface_format(),
            "explicit effect",
            effect_source,
            effect_layout,
        )
        .unwrap();
    let mut frame = scene(vec![usage], template);
    // Post-processing happens before HUD; use a world primitive for this assertion.
    frame.world.append(&mut frame.hud);
    frame.camera_pos = utils::math::Vec2::new(2., 1.);
    frame
        .post_process
        .push(client::rendering::PostProcessSubmission {
            effect,
            parameters: bytemuck::bytes_of(&1f32).to_vec(),
        });
    renderer
        .render_to_texture(&context, &mut assets, &mut data, frame, &output)
        .unwrap();
    for pixel in pixels(&context, &output).chunks_exact(4) {
        assert_eq!(pixel, [255, 0, 255, 255]);
    }
    assert!(context.take_error().is_none());
}
