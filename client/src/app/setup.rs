use super::App;
use crate::{
    graphic_data::post_process_effect_type as effects,
    rendering::{RenderAssets, SceneRenderer},
};

impl App {
    pub(super) fn initialize_renderer(
        &mut self,
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
    ) -> Result<SceneRenderer, String> {
        let mut renderer =
            SceneRenderer::new(context, assets).map_err(|error| error.to_string())?;
        renderer
            .register_vfx_post_effect(
                context,
                assets,
                "hit_flash",
                include_str!("../graphic_data/shader/hit_flash_effect.frag.wgsl"),
                |input| effects::HitFlashUniform {
                    intensity: input.weight,
                },
            )
            .map_err(|error| error.to_string())?;
        renderer
            .register_vfx_post_effect(
                context,
                assets,
                "blind",
                include_str!("../graphic_data/shader/blind_effect.frag.wgsl"),
                |input| effects::BlindUniform {
                    amount: input.weight,
                    aspect_ratio: input.aspect_ratio(),
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(renderer)
    }

    pub(super) fn initialize_health_material(
        &mut self,
        context: &prism::GpuContext,
        assets: &mut RenderAssets,
    ) -> Result<prism_legacy::ids::MaterialId, String> {
        let id = assets
            .create_health_material(context)
            .map_err(|error| error.to_string())?;
        self.id_register
            .insert(crate::key::material::HP_MATERIAL, id);
        Ok(id)
    }
}
