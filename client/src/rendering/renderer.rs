//! Alpha scene orchestration using only native Velum resource and draw APIs.
use super::post_process::PostProcessId;
use super::shader_bindings::{self, EffectBindings};
use super::{AssetCache, SceneFrame};
use velum::{
    OwnedResource, RenderData, RenderDataId, SharedResource,
    backend::{
        self as gpu, RenderError, RenderResult,
        bindings::BindingResource,
        material::{self, Material, MaterialId, MaterialTemplate, MaterialTemplateId, ShaderEntry},
        object::RenderObject,
        target,
    },
};

type Context = super::GpuContext;
pub(super) fn render_data_error(error: velum::RenderDataError) -> RenderError {
    RenderError::new("render data", error.to_string())
}
struct EffectProgram {
    template: RenderDataId<SharedResource<MaterialTemplateId>>,
    layout: EffectBindings,
}
pub struct AlphaRenderer {
    scenes: super::scenes::Scenes,
    post_process: super::post_process::PostProcessRegistry,
    effects: Vec<EffectProgram>,
    blit: Option<RenderDataId<SharedResource<MaterialTemplateId>>>,
    blit_layout: EffectBindings,
}
fn effect_template(
    context: Option<gpu::context::ContextId>,
    format: wgpu::TextureFormat,
    label: &str,
    fragment: &str,
    layout: EffectBindings,
) -> RenderResult<SharedResource<MaterialTemplateId>> {
    let vertex = gpu::try_shader(
        context,
        "post vertex",
        include_str!("../graphic_data/shader/default_post_process.vert.wgsl"),
    )?;
    let fragment = gpu::try_shader(context, label, fragment)?;
    let mut template = MaterialTemplate::fullscreen(context, label, vertex, format);
    template.fragment = Some(ShaderEntry::new(fragment, "fs_main"));
    template.schema = layout.schema()?;
    velum::try_material_template!(template).map(OwnedResource::share)
}
impl AlphaRenderer {
    pub fn new(
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
    ) -> RenderResult<Self> {
        Ok(Self {
            scenes: super::scenes::Scenes::new()?,
            post_process: Default::default(),
            effects: vec![],
            blit: if context.context_id().is_some() {
                Some(data.insert(effect_template(
                    assets.context,
                    context.surface_format(),
                    "presentation",
                    include_str!("../graphic_data/shader/default_post_process.frag.wgsl"),
                    EffectBindings::bundled(0),
                )?))
            } else {
                None
            },
            blit_layout: EffectBindings::bundled(0),
        })
    }
    pub fn activate(&mut self, screen: &crate::core::screen::AppScreen) -> RenderResult<()> {
        self.scenes.activate(screen)
    }
    pub fn register_post_effect(
        &mut self,
        assets: &mut AssetCache,
        data: &mut RenderData,
        format: wgpu::TextureFormat,
        label: &str,
        source: &str,
        parameter_size: u64,
    ) -> RenderResult<PostProcessId> {
        self.register_post_effect_with_bindings(
            assets,
            data,
            format,
            label,
            source,
            EffectBindings::bundled(parameter_size),
        )
    }
    pub fn register_post_effect_with_bindings(
        &mut self,
        assets: &mut AssetCache,
        data: &mut RenderData,
        format: wgpu::TextureFormat,
        label: &str,
        source: &str,
        layout: EffectBindings,
    ) -> RenderResult<PostProcessId> {
        let parameter_size = layout.parameters.map_or(0, |slot| slot.size);
        if parameter_size % 4 != 0 {
            return Err(RenderError::new(
                "post-process uniform",
                "uniform size must be a multiple of four bytes",
            ));
        }
        let template = effect_template(assets.context, format, label, source, layout)?;
        let id = PostProcessId(self.effects.len() as u64);
        self.effects.push(EffectProgram {
            template: data.insert(template),
            layout,
        });
        Ok(id)
    }
    pub fn register_vfx_post_effect<T: bytemuck::Pod + 'static>(
        &mut self,
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        material: &str,
        source: &str,
        encode: impl Fn(&super::PostProcessInput<'_>) -> T + 'static,
    ) -> RenderResult<PostProcessId> {
        self.post_process.check_material(material)?;
        let id = self.register_post_effect(
            assets,
            data,
            context.surface_format(),
            material,
            source,
            std::mem::size_of::<T>() as u64,
        )?;
        self.post_process.register(material.to_owned(), id, encode);
        Ok(id)
    }
    pub fn add_post_process_pass<T: bytemuck::Pod>(
        &mut self,
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        source: &str,
        value: Option<T>,
    ) -> RenderResult<PostProcessId> {
        let bytes = value
            .map(|value| bytemuck::bytes_of(&value).to_vec())
            .unwrap_or_default();
        let id = self.register_post_effect(
            assets,
            data,
            context.surface_format(),
            "post-process pass",
            source,
            bytes.len() as u64,
        )?;
        self.post_process.add_manual(id, bytes);
        Ok(id)
    }
    pub fn enable_post_process_pass(&mut self, id: PostProcessId) -> RenderResult<()> {
        self.post_process.enable(id, true)
    }
    pub fn disable_post_process_pass(&mut self, id: PostProcessId) -> RenderResult<()> {
        self.post_process.enable(id, false)
    }
    pub fn write_post_process_uniform<T: bytemuck::Pod>(
        &mut self,
        id: PostProcessId,
        value: T,
    ) -> RenderResult<()> {
        self.post_process.write(id, value)
    }

    fn fullscreen(
        context: Option<gpu::context::ContextId>,
        source: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        template: SharedResource<MaterialTemplateId>,
        layout: EffectBindings,
        parameters: &[u8],
    ) -> RenderResult<OwnedResource<MaterialId>> {
        let mut bindings = shader_bindings::register_sets(
            context,
            [
                (
                    layout.source.texture,
                    BindingResource::Texture(source.clone()),
                ),
                (
                    layout.source.sampler,
                    BindingResource::Sampler(sampler.clone()),
                ),
            ],
        )?;
        if let Some(slot) = layout.parameters {
            bindings.push(shader_bindings::uniform_set(context, slot, parameters)?);
        } else if !parameters.is_empty() {
            return Err(RenderError::new("post-process", "unexpected parameters"));
        }
        material::try_register_in(Material { template, bindings }, context)
    }
    pub fn render_to_texture(
        &mut self,
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        scene: SceneFrame,
        target: &wgpu::Texture,
    ) -> RenderResult<gpu::report::FrameReport> {
        if target.format() != context.surface_format() {
            return Err(RenderError::new(
                "target",
                "texture format differs from the Alpha material format",
            ));
        }
        if target.sample_count() != 1 {
            return Err(RenderError::new(
                "target",
                "Alpha materials require a single-sampled target",
            ));
        }
        let mut frame = gpu::renderer::OffscreenFrame::new(
            assets.context,
            target.create_view(&Default::default()),
        )?;

        self.prepare_scene(
            context,
            assets,
            data,
            scene,
            [target.width(), target.height()],
        )?;
        let active = self.scenes.active_mut()?;
        frame
            .execute(&mut active.renderer, &mut active.data)
            .map_err(|e| RenderError::new("scene execution", e.to_string()))?;
        Ok(frame.finish())
    }

    fn prepare_scene(
        &mut self,
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        mut scene: SceneFrame,
        size: [u32; 2],
    ) -> RenderResult<()> {
        let mut submissions = self.post_process.submissions(&scene.post_effects, size);

        submissions.append(&mut scene.post_process);

        let sampler = data
            .require(assets.sampler)
            .map_err(render_data_error)?
            .as_raw()
            .clone();
        let (active, passes) = self.scenes.prepare(context, size, submissions.len())?;
        let [world, vfx, hud] = active
            .data
            .require_mut(passes.geometry)
            .map_err(render_data_error)?
            .prepare(assets, data, &mut scene, size[0], size[1])?;
        // Resolve all objects before replacing phase contents. An invalid effect can be retried.
        let geometry = active
            .data
            .require(passes.geometry)
            .map_err(render_data_error)?
            .fullscreen
            .id();
        let mut materials = Vec::with_capacity(passes.targets.len());
        let material_count = submissions.len() + usize::from(passes.present.is_none());
        for (index, target_id) in passes.targets.iter().take(material_count).enumerate() {
            let source = target::get(
                active
                    .data
                    .require(*target_id)
                    .map_err(render_data_error)?
                    .id(),
            )
            .ok_or_else(|| RenderError::new("target", "missing scene target"))?;
            let (template, layout, parameters) = if let Some(submission) = submissions.get(index) {
                let program = self
                    .effects
                    .get(submission.effect.0 as usize)
                    .ok_or_else(|| RenderError::new("post-process", "unknown effect"))?;
                (
                    program.template,
                    program.layout,
                    submission.parameters.as_slice(),
                )
            } else {
                (
                    self.blit.ok_or_else(|| {
                        RenderError::new("presentation", "missing offscreen template")
                    })?,
                    self.blit_layout,
                    &[][..],
                )
            };
            materials.push(Self::fullscreen(
                assets.context,
                source.texture().view(),
                &sampler,
                data.require(template).map_err(render_data_error)?.clone(),
                layout,
                parameters,
            )?);
        }
        let fullscreen = materials
            .iter()
            .map(|material| vec![RenderObject::new(geometry, material.id())]);
        let draws = [world, vfx].into_iter().chain(fullscreen).chain([hud]);
        for (&phase, objects) in passes.phases.iter().zip(draws) {
            let phase = active.data.require_mut(phase).map_err(render_data_error)?;
            phase.clear();
            for object in objects {
                phase.push(object);
            }
        }
        // Glyph layout consumes text in painter order. Geometry is sorted by the native phase.
        scene.hud.sort_by_key(super::Primitive::layer);
        let has_text = active
            .data
            .require_mut(passes.text)
            .map_err(render_data_error)?
            .prepare(context, size, &scene.hud)?;
        if has_text {
            active.renderer.activate_pass(passes.text_pass);
        } else {
            active.renderer.deactivate_pass(passes.text_pass);
        }
        let owners = active
            .data
            .require_mut(passes.geometry)
            .map_err(render_data_error)?;
        owners.materials.extend(materials);

        Ok(())
    }
    pub fn render_to_target(
        &mut self,
        context: &Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        scene: SceneFrame,
        id: super::RenderTargetId,
    ) -> RenderResult<gpu::report::FrameReport> {
        let target = target::get(id).ok_or_else(|| RenderError::new("target", "missing target"))?;
        self.render_to_texture(context, assets, data, scene, target.texture().as_raw())
    }

    pub fn render<'a>(
        &mut self,
        context: &'a Context,
        assets: &mut AssetCache,
        data: &mut RenderData,
        scene: SceneFrame,
    ) -> RenderResult<Option<gpu::renderer::Frame<'a>>> {
        let Some(mut frame) = context.begin_frame()? else {
            return Ok(None);
        };
        self.prepare_scene(context, assets, data, scene, frame.size_in_pixels())?;
        self.scenes
            .velum
            .render(&mut frame)
            .map_err(|e| RenderError::new("scene execution", e.to_string()))?;
        Ok(Some(frame))
    }
}
