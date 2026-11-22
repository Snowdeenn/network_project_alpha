use super::App;
use crate::{
    graphic_data::post_process_effect_type as effects,
    rendering::{AssetCache, SceneRenderer},
};

impl App {
    pub(super) fn initialize_renderer(
        &mut self,
        context: &crate::rendering::GpuContext,
        assets: &mut AssetCache,
        data: &mut velum::RenderData,
    ) -> Result<SceneRenderer, String> {
        let mut renderer =
            SceneRenderer::new(context, assets, data).map_err(|error| error.to_string())?;
        renderer
            .register_vfx_post_effect(
                context,
                assets,
                data,
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
                data,
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
        context: &crate::rendering::GpuContext,
        bridge: &mut crate::rendering::nodus_bridge::NodusBridge,
        data: &mut velum::RenderData,
    ) -> Result<prism_legacy::ids::MaterialId, String> {
        let template =
            crate::rendering::create_health_material(context).map_err(|error| error.to_string())?;
        let id = bridge.register_material(
            data,
            template,
            crate::rendering::shader_bindings::HEALTH_BINDINGS,
        );
        self.resource
            .write_resource::<utils::ids::Register>()
            .insert(crate::key::material::HP_MATERIAL, id);
        Ok(id)
    }
}
