//! Run explicitly on machines with a graphics adapter:
//! cargo test -p client --test prism_v2_rendering -- --ignored
use client::rendering::{BlendMode, Color, Primitive, RenderAssets, SceneFrame, SceneRenderer};
use prism::frame::v2::PostProcessSubmission;
use utils::math::Vec2;

fn quad(pos: [f32; 2], size: [f32; 2], color: [f32; 4], layer: u8) -> Primitive {
    Primitive::Shape {
        shape: prism::Shape::Quad {
            pos,
            size,
            color,
            rotation: 0.0,
            uv: None,
        },
        blend: BlendMode::Alpha,
        layer,
    }
}

fn pixels(context: &prism::GpuContext, target: &wgpu::Texture) -> Vec<u8> {
    let row = (target.width() * 4).div_ceil(256) * 256;
    let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size: row as u64 * target.height() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = context.create_encoder("test readback");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(target.height()),
            },
        },
        target.size(),
    );
    context.submit(encoder);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    let mut pixels = vec![];
    for line in mapped.chunks(row as usize) {
        pixels.extend_from_slice(&line[..target.width() as usize * 4]);
    }
    pixels
}

fn target(context: &prism::GpuContext, width: u32, height: u32) -> wgpu::Texture {
    context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("client V2 test"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn v2_renders_camera_layers_vfx_ui_text_effects_and_resize() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("graphics adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let mut context =
        prism::GpuContext::from_device(device, queue, 64, 64, wgpu::TextureFormat::Rgba8UnormSrgb)
            .unwrap();
    let mut assets = RenderAssets::new(&context).unwrap();
    let mut renderer = SceneRenderer::new(&context, &mut assets).unwrap();
    let health = assets.create_health_material(&context).unwrap();
    let output = target(&context, 64, 64);
    let mut scene = SceneFrame::new();
    scene.camera_pos = Vec2::new(100.0, 100.0);
    scene.cam_shake_offset = Vec2::new(4.0, 0.0);
    scene.push_world(quad([96.0, 96.0], [8.0; 2], [1.0, 0.0, 0.0, 1.0], 3));
    scene.push_world(quad([96.0, 96.0], [8.0; 2], [0.0, 1.0, 0.0, 1.0], 1));
    scene.push_vfx(Primitive::Shape {
        shape: prism::Shape::Quad {
            pos: [96.0, 96.0],
            size: [8.0; 2],
            rotation: 0.0,
            uv: None,
            color: [0.0, 0.0, 1.0, 0.5],
        },
        blend: BlendMode::Additive,
        layer: 0,
    });
    let mut ui = nodus::DrawCommandBuffer::new(4);
    ui.push(nodus::DrawCommand::Material {
        material_id: health,
        texture_id: None,
        pos: Vec2::zero(),
        size: Vec2::new(8.0, 8.0),
        tint: Color::WHITE,
        uniform_data: bytemuck::bytes_of(&1.0f32).to_vec(),
        layer: 0,
    });
    ui.push(nodus::DrawCommand::Text {
        text: "V2".into(),
        pos: Vec2::new(12.0, 46.0),
        font_size: 12.0,
        color: Color::WHITE,
        layer: 1,
    });

    let path = std::env::temp_dir().join(format!(
        "client-v2-texture-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255u8; 4]))
        .save(&path)
        .unwrap();
    let texture = assets.load_texture(&context, &path).unwrap();
    assert_eq!(
        texture,
        assets.load_texture(&context, &path).unwrap(),
        "texture cache"
    );
    std::fs::remove_file(path).unwrap();
    let ui_texture = assets.register_nodus_texture(texture).unwrap();
    ui.push(nodus::DrawCommand::Texture {
        texture_id: ui_texture,
        pos: Vec2::new(48.0, 0.0),
        size: Vec2::new(8.0, 8.0),
        tint: Color::WHITE,
        layer: 0,
    });
    ui.push(nodus::DrawCommand::NinePatch {
        texture_id: ui_texture,
        pos: Vec2::new(48.0, 12.0),
        size: Vec2::new(8.0, 8.0),
        margins: prism_legacy::NinePatchMargins::uniform(0.5),
        tint: Color::WHITE,
        layer: 0,
    });
    client::rendering::nodus_bridge::collect(&ui, &assets, &mut scene);
    let report = renderer
        .render_to_texture(&context, &mut assets, scene, &output)
        .unwrap();
    assert!(
        report.skipped_objects.is_empty(),
        "{:?}",
        report.skipped_objects
    );
    let image = pixels(&context, &output);
    let pixel = |x: usize, y: usize| &image[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert_eq!(
        pixel(4, 4),
        [0, 255, 0, 255],
        "HUD health stays in screen coordinates"
    );
    assert_eq!(pixel(50, 4), [255; 4], "textured UI");
    assert_eq!(pixel(50, 14), [255; 4], "nine-patch UI");
    assert!(
        pixel(36, 32)[0] > 240 && pixel(36, 32)[1] == 0 && pixel(36, 32)[2] > 100,
        "world camera, stable layer sorting and additive VFX: {:?}",
        pixel(36, 32)
    );
    assert_eq!(pixel(28, 32), [0, 0, 0, 255], "camera shake is applied");
    assert!(
        (46..64).any(|y| (12..36).any(|x| pixel(x, y)[0] > 0)),
        "Glyphon text"
    );

    let flash = renderer
        .register_post_effect(
            &mut assets,
            context.surface_format(),
            "flash",
            include_str!("../src/graphic_data/shader/hit_flash_effect.frag.wgsl"),
            4,
        )
        .unwrap();
    let blind = renderer
        .register_post_effect(
            &mut assets,
            context.surface_format(),
            "blind",
            include_str!("../src/graphic_data/shader/blind_effect.frag.wgsl"),
            8,
        )
        .unwrap();
    let mut scene = SceneFrame::new();
    scene.push_hud(quad([0.0; 2], [8.0; 2], [0.0, 1.0, 0.0, 1.0], 0));
    scene.post_process = vec![
        PostProcessSubmission {
            effect: flash,
            parameters: bytemuck::bytes_of(&1.0f32).to_vec(),
        },
        PostProcessSubmission {
            effect: blind,
            parameters: bytemuck::cast_slice(&[1.0f32, 1.0]).to_vec(),
        },
    ];
    let report = renderer
        .render_to_texture(&context, &mut assets, scene, &output)
        .unwrap();
    assert!(
        report.skipped_objects.is_empty(),
        "{:?}",
        report.skipped_objects
    );
    let image = pixels(&context, &output);
    assert_eq!(
        &image[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        [255, 0, 0, 255],
        "flash at center"
    );
    assert_eq!(
        &image[(4 * 64 + 4) * 4..(4 * 64 + 4) * 4 + 4],
        [0, 255, 0, 255],
        "HUD after effects"
    );
    assert_eq!(
        &image[(60 * 64 + 60) * 4..(60 * 64 + 60) * 4 + 4],
        [0, 0, 0, 255],
        "blind edge"
    );

    context.resize(96, 32);
    let resized = target(&context, 96, 32);
    let mut crowded = SceneFrame::new();
    for _ in 0..2048 {
        crowded.push_hud(quad([0.0; 2], [96.0, 32.0], [0.0, 1.0, 0.0, 1.0], 0));
    }
    let report = renderer
        .render_to_texture(&context, &mut assets, crowded, &resized)
        .unwrap();
    assert!(
        report.skipped_objects.is_empty(),
        "buffer growth: {:?}",
        report.skipped_objects
    );
    assert!(
        report.draw_calls <= 3,
        "consecutive compatible draws must batch: {}",
        report.draw_calls
    );
    assert!(
        pixels(&context, &resized)
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 255, 0, 255])
    );
    let report = renderer
        .render_to_texture(&context, &mut assets, SceneFrame::new(), &resized)
        .unwrap();
    assert!(report.skipped_objects.is_empty());
    assert!(
        pixels(&context, &resized)
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 0, 255]),
        "no stale geometry or text after resize"
    );
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn migrated_gameplay_effects_render_through_vfx_system_and_prism_v2() {
    use client::graphic_data::post_process_effect_type::{BlindUniform, HitFlashUniform};
    use client::vfx::{VfxBudgets, VfxSystem, render_bridge};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("graphics adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let context =
        prism::GpuContext::from_device(device, queue, 64, 64, wgpu::TextureFormat::Rgba8UnormSrgb)
            .unwrap();
    let mut assets = RenderAssets::new(&context).unwrap();
    let mut renderer = SceneRenderer::new(&context, &mut assets).unwrap();
    let mut vfx = VfxSystem::with_seed(VfxBudgets::default(), 42);
    vfx.registry.register_gameplay_effects().unwrap();
    let api = vfx.api();
    api.slash(Vec2::new(32.0, 32.0), Vec2::new(1.0, 0.0), 16.0, 16.0)
        .unwrap();
    api.dash_ghost(Vec2::zero(), 0.2, Color::BLUE).unwrap();
    api.trail_point(7, Vec2::new(8.0, 52.0), Color::RED)
        .unwrap();
    api.trail_point(7, Vec2::new(56.0, 52.0), Color::RED)
        .unwrap();
    api.entity_flash(7, 0.2).unwrap();
    vfx.update(1.0 / 60.0, |_| Some(Vec2::zero()));
    let mut scene = SceneFrame::new();
    scene.camera_pos = Vec2::new(32.0, 32.0);
    render_bridge::append_to_frame(&vfx.extract(), &mut scene);

    let path = std::env::temp_dir().join(format!(
        "client-vfx-mask-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let texture_image =
        image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 0]).unwrap();
    texture_image.save(&path).unwrap();
    let texture = assets.load_texture(&context, &path).unwrap();
    std::fs::remove_file(path).unwrap();
    let tint = vfx.entity_tint(7).unwrap();
    scene.push_world(Primitive::Texture {
        flash: true,
        id: texture,
        pos: [48.0, 0.0],
        size: [16.0, 8.0],
        rotation: 0.0,
        uv: None,
        tint,
        blend: BlendMode::Alpha,
        layer: 1,
    });
    let output = target(&context, 64, 64);
    let report = renderer
        .render_to_texture(&context, &mut assets, scene, &output)
        .unwrap();
    assert!(
        report.skipped_objects.is_empty(),
        "{:?}",
        report.skipped_objects
    );
    let image = pixels(&context, &output);
    let pixel = |x: usize, y: usize| &image[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert!(
        pixel(44, 32)[0] > 230 && pixel(44, 32)[1] > 230,
        "slash arc: {:?}",
        pixel(44, 32)
    );
    assert!(
        pixel(4, 4)[2] > 100 && pixel(4, 4)[0] == 0,
        "blue ghost: {:?}",
        pixel(4, 4)
    );
    assert_eq!(pixel(32, 52), [255, 0, 0, 255], "manual sword trail");
    assert_eq!(pixel(50, 4), [255; 4], "entity flash replaces sprite RGB");
    assert_eq!(
        pixel(62, 4),
        [0, 0, 0, 255],
        "entity flash preserves sprite alpha"
    );

    renderer
        .register_vfx_post_effect(
            &context,
            &mut assets,
            "hit_flash",
            include_str!("../src/graphic_data/shader/hit_flash_effect.frag.wgsl"),
            |input| HitFlashUniform {
                intensity: input.weight,
            },
        )
        .unwrap();
    renderer
        .register_vfx_post_effect(
            &context,
            &mut assets,
            "blind",
            include_str!("../src/graphic_data/shader/blind_effect.frag.wgsl"),
            |input| BlindUniform {
                amount: input.weight,
                aspect_ratio: input.aspect_ratio(),
            },
        )
        .unwrap();
    api.player_hit().unwrap();
    for _ in 0..6 {
        vfx.update(1.0 / 60.0, |_| Some(Vec2::zero()));
    }
    let mut scene = SceneFrame::new();
    scene.post_effects = vfx.extract().post_process;
    let report = renderer
        .render_to_texture(&context, &mut assets, scene, &output)
        .unwrap();
    assert!(report.skipped_objects.is_empty());
    let image = pixels(&context, &output);
    let center = &image[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4];
    assert!(
        (180..=195).contains(&center[0]) && center[1] == 0 && center[2] == 0,
        "screen flash uses the runtime envelope: {center:?}"
    );
}

#[test]
#[ignore = "requires a GPU or software graphics adapter"]
fn arbitrary_post_effects_need_only_a_shader_and_registration_and_keep_manual_pass_controls() {
    use client::vfx::{SpawnOptions, VfxSpawnParams, VfxSystem, registry::Priority, render_bridge};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("graphics adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let context =
        prism::GpuContext::from_device(device, queue, 64, 64, wgpu::TextureFormat::Rgba8UnormSrgb)
            .unwrap();
    let mut assets = RenderAssets::new(&context).unwrap();
    let mut renderer = SceneRenderer::new(&context, &mut assets).unwrap();
    let shader = r#"
        @group(1) @binding(0) var<uniform> tint: vec4<f32>;
        @fragment fn fs_main() -> @location(0) vec4<f32> { return tint; }
    "#;
    let custom = renderer
        .register_vfx_post_effect(&context, &mut assets, "custom/tint", shader, |input| {
            let color = input.vector("color").unwrap();
            let strength = input.scalar("strength").unwrap() * input.weight;
            [
                color[0] * strength,
                color[1] * strength,
                color[2] * strength,
                1.0,
            ]
        })
        .unwrap();
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../assets/vfx/exemples_vfx/post-process.json"
    ))
    .unwrap();
    document["name"] = serde_json::json!("a_totally_new_effect");
    document["visual"]["representations"][0]["material"] = serde_json::json!("custom/tint");
    document["visual"]["appearance"]["weight"] = serde_json::json!(0.5);
    document["visual"]["appearance"]["parameters"] =
        serde_json::json!({"color": [0.0, 1.0, 0.0], "strength": 1.0});
    document["visual"]["transitions"] = serde_json::json!([]);
    let mut vfx = VfxSystem::default();
    vfx.registry
        .register_json(&document.to_string(), Priority::Normal)
        .unwrap();
    vfx.api()
        .spawn_detached(
            "a_totally_new_effect",
            VfxSpawnParams::default(),
            SpawnOptions::default(),
        )
        .unwrap();
    vfx.update(1.0 / 60.0, |_| None);
    assert_eq!(vfx.post_process("custom/tint").weight, 0.5);
    let output = target(&context, 64, 64);
    let frame = |vfx: &VfxSystem| {
        let mut frame = SceneFrame::new();
        render_bridge::append_to_frame(&vfx.extract(), &mut frame);
        frame
    };
    let report = renderer
        .render_to_texture(&context, &mut assets, frame(&vfx), &output)
        .unwrap();
    assert!(report.skipped_objects.is_empty());
    let image = pixels(&context, &output);
    assert!(
        (180..=195).contains(&image[1]) && image[0] == 0 && image[2] == 0,
        "custom uniforms: {:?}",
        &image[..4]
    );
    renderer.disable_post_process_pass(custom).unwrap();
    renderer
        .render_to_texture(&context, &mut assets, frame(&vfx), &output)
        .unwrap();
    assert_eq!(&pixels(&context, &output)[..4], [0, 0, 0, 255]);
    renderer.enable_post_process_pass(custom).unwrap();
    renderer
        .render_to_texture(&context, &mut assets, frame(&vfx), &output)
        .unwrap();
    assert!(pixels(&context, &output)[1] > 180);
    renderer.disable_post_process_pass(custom).unwrap();

    let manual = renderer
        .add_post_process_pass(&context, &mut assets, shader, Some([0.0f32, 0.0, 1.0, 1.0]))
        .unwrap();
    renderer
        .render_to_texture(&context, &mut assets, SceneFrame::new(), &output)
        .unwrap();
    assert_eq!(&pixels(&context, &output)[..4], [0, 0, 255, 255]);
    renderer
        .write_post_process_uniform(manual, [1.0f32, 0.0, 0.0, 1.0])
        .unwrap();
    renderer
        .render_to_texture(&context, &mut assets, SceneFrame::new(), &output)
        .unwrap();
    assert_eq!(&pixels(&context, &output)[..4], [255, 0, 0, 255]);
    renderer.disable_post_process_pass(manual).unwrap();
    renderer
        .render_to_texture(&context, &mut assets, SceneFrame::new(), &output)
        .unwrap();
    assert_eq!(&pixels(&context, &output)[..4], [0, 0, 0, 255]);
}
