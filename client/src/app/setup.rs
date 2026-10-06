use super::App;
use crate::graphic_data::post_process_effect_type;
use crate::key::{material, post, shader};
use prism::ids::{MaterialId, ShaderId};

impl App {
    fn shader_id(&self, key: &'static str) -> ShaderId {
        self.id_register
            .get(key)
            .expect("Le shader doit être chargé avant la création du renderer")
    }

    fn load_shaders(
        &mut self,
        gpu_ctx: &prism::GpuContext,
        gpu_resources: &mut prism::GpuResources,
    ) -> Result<(), String> {
        let shaders = [
            (shader::DEFAULT_VERTEX, "default.vert.wgsl"),
            (shader::DEFAULT_FRAGMENT, "default.frag.wgsl"),
            (post::DEFAULT_POST_VERTEX, "default_post_process.vert.wgsl"),
            (
                post::DEFAULT_POST_FRAGMENT,
                "default_post_process.frag.wgsl",
            ),
            (shader::TEXTURED_VERTEX, "default_textured.vert.wgsl"),
            (shader::TEXTURED_FRAGMENT, "default_textured.frag.wgsl"),
            (post::HIT_FLASH_FRAG, "hit_flash_effect.frag.wgsl"),
            (post::BLIND_FRAG, "blind_effect.frag.wgsl"),
        ];
        for (key, file) in shaders {
            let path = format!("client/src/graphic_data/shader/{file}");
            let id = gpu_resources
                .load_shader(gpu_ctx, &path)
                .map_err(|error| format!("Erreur lors du chargement du shader {file} : {error}"))?;
            self.id_register.insert(key, id);
        }
        Ok(())
    }

    pub(super) fn initialize_renderer(
        &mut self,
        gpu_ctx: &prism::GpuContext,
        gpu_resources: &mut prism::GpuResources,
    ) -> Result<prism::Renderer, String> {
        self.load_shaders(gpu_ctx, gpu_resources)?;
        let mut renderer = prism::Renderer::new(
            gpu_ctx,
            gpu_resources,
            self.shader_id(shader::DEFAULT_VERTEX),
            self.shader_id(shader::DEFAULT_FRAGMENT),
            self.shader_id(shader::TEXTURED_VERTEX),
            self.shader_id(shader::TEXTURED_FRAGMENT),
        )
        .map_err(|error| format!("Échec de l'initialisation du renderer Prism : {error}"))?;
        self.initialize_post_processing(gpu_ctx, gpu_resources, &mut renderer)?;
        Ok(renderer)
    }

    fn initialize_post_processing(
        &mut self,
        gpu_ctx: &prism::GpuContext,
        gpu_resources: &prism::GpuResources,
        renderer: &mut prism::Renderer,
    ) -> Result<(), String> {
        let vertex_shader = self.shader_id(post::DEFAULT_POST_VERTEX);
        renderer
            .add_post_process_pass::<()>(
                gpu_ctx,
                gpu_resources,
                vertex_shader,
                self.shader_id(post::DEFAULT_POST_FRAGMENT),
                None,
            )
            .map_err(|error| {
                format!("Erreur lors de la création de la passe par défaut : {error}")
            })?;

        let uniform = post_process_effect_type::HitFlashUniform { intensity: 0.5 };
        let hit_flash_id = renderer
            .add_post_process_pass(
                gpu_ctx,
                gpu_resources,
                vertex_shader,
                self.shader_id(post::HIT_FLASH_FRAG),
                Some(uniform),
            )
            .map_err(|error| {
                format!("Erreur lors de la création de la passe hit flash : {error}")
            })?;
        renderer.disable_post_process_pass(hit_flash_id);
        self.resource
            .insert(post_process_effect_type::HitFlashEffect {
                id: hit_flash_id,
                timer: 0.0,
                total_duration: 0.2,
                intensity: uniform.intensity,
            });

        let aspect_ratio = gpu_ctx.size.width as f32 / gpu_ctx.size.height as f32;
        let blind_uniform = post_process_effect_type::BlindUniform {
            amount: 1.0,
            aspect_ratio,
        };
        let blind_id = renderer
            .add_post_process_pass(
                gpu_ctx,
                gpu_resources,
                vertex_shader,
                self.shader_id(post::BLIND_FRAG),
                Some(blind_uniform),
            )
            .map_err(|error| format!("Erreur lors de la création de la passe blind : {error}"))?;
        renderer.disable_post_process_pass(blind_id);
        self.resource.insert(post_process_effect_type::BlindEffect {
            id: blind_id,
            timer: 0.0,
            total_duration: 0.0,
            aspect_ratio,
            amount: 0.0,
        });
        Ok(())
    }

    pub(super) fn initialize_health_material(
        &mut self,
        gpu_ctx: &prism::GpuContext,
        gpu_resources: &mut prism::GpuResources,
        renderer: &mut prism::Renderer,
    ) -> Result<MaterialId, String> {
        let fragment_shader = gpu_resources
            .load_shader(
                gpu_ctx,
                "client/src/graphic_data/shader/progress_bar.frag.wgsl",
            )
            .map_err(|error| format!("Erreur lors du chargement du shader de vie : {error}"))?;
        self.id_register
            .insert("shader/progress_bar_frag", fragment_shader);
        let pipeline = renderer
            .create_pipeline(
                gpu_ctx,
                gpu_resources,
                prism::PipelineKey {
                    vertex_shader: self.shader_id(shader::TEXTURED_VERTEX),
                    fragment_shader,
                    blend_mode: prism::BlendMode::Alpha,
                    vertex_format: prism::VertexFormat::Pos2UvColor,
                    bind_groups: &prism::MATERIAL_BIND_GROUP,
                },
            )
            .map_err(|error| format!("Impossible de créer la pipeline de vie : {error}"))?;
        // Le ratio de vie est transmis dans le scratch buffer.
        let id = gpu_resources.create_material(pipeline, vec![], std::mem::size_of::<f32>());
        self.id_register.insert(material::HP_MATERIAL, id);
        Ok(id)
    }
}
