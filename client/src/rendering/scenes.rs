use super::GpuContext;
use velum::backend::{
    self as gpu, RenderResult,
    object::{PhaseId, PhaseOperation, RenderPhase},
    pass::{self, PassTarget},
    target::{self, RenderTargetDescriptor, RenderTargetId, TargetSize},
};
use velum::{RenderDataId, Scene, SceneId, Velum};

/// Handles into one native scene's data. One intermediate target feeds each fullscreen pass.
pub(super) struct ScenePasses {
    size: [u32; 2],
    pub targets: Vec<RenderDataId<velum::OwnedResource<RenderTargetId>>>,
    pub present: Option<RenderDataId<velum::effects::PresentEffect>>,
    pub text_pass: velum::scene_renderer::PassId,
    pub geometry: RenderDataId<super::draw::SceneGeometry>,
    pub text: RenderDataId<super::text::TextOverlay>,
    pub phases: Vec<RenderDataId<RenderPhase>>,
}

pub(super) struct Scenes {
    pub(super) velum: Velum,
    screens: [(SceneId, Option<ScenePasses>); 3],
}

impl Scenes {
    pub fn new() -> RenderResult<Self> {
        let mut velum = Velum::new();
        let screens = std::array::from_fn(|_| {
            (
                velum.add_scene(Scene {
                    renderer: Default::default(),
                    data: Default::default(),
                }),
                None,
            )
        });
        velum
            .activate(screens[0].0)
            .map_err(|e| gpu::RenderError::new("activate scene", e.to_string()))?;
        Ok(Self { velum, screens })
    }

    pub fn activate(&mut self, screen: &crate::core::screen::AppScreen) -> RenderResult<()> {
        use crate::core::screen::AppScreen::*;
        let index = match screen {
            MainMenu => 0,
            Lobby(_) => 1,
            InGame => 2,
        };
        self.velum
            .activate(self.screens[index].0)
            .map_err(|e| gpu::RenderError::new("activate scene", e.to_string()))
    }

    pub fn active_mut(&mut self) -> RenderResult<&mut Scene> {
        self.velum
            .active_scene()
            .and_then(|id| self.velum.scene_mut(id))
            .ok_or_else(|| gpu::RenderError::new("scene", "missing active scene"))
    }

    pub fn prepare(
        &mut self,
        gpu: &GpuContext,
        size: [u32; 2],
        effects: usize,
    ) -> RenderResult<(&mut Scene, &ScenePasses)> {
        let id = self.velum.active_scene().expect("initialized active scene");
        let (_, cached) = self
            .screens
            .iter_mut()
            .find(|(screen, _)| *screen == id)
            .expect("registered screen");
        let scene = self.velum.scene_mut(id).expect("registered scene");
        if cached
            .as_ref()
            .is_none_or(|p| p.size != size || p.targets.len() != effects + 1)
        {
            // All handles below belong to this schedule, including its target owners.
            scene.renderer = Default::default();
            let (geometry, text) = if let Some(old) = cached.take() {
                if let Some(id) = old.present {
                    scene.data.remove(id);
                }
                for id in old.phases {
                    scene.data.remove(id);
                }
                for id in old.targets {
                    scene.data.remove(id);
                }
                (old.geometry, old.text)
            } else {
                let geometry = super::draw::SceneGeometry::new(gpu.context_id())?;
                (
                    scene.data.insert(geometry),
                    scene.data.insert(super::text::TextOverlay::new(gpu)),
                )
            };
            let mut targets = Vec::with_capacity(effects + 1);
            for _ in 0..=effects {
                let owner = target::register(
                    gpu.context_id(),
                    RenderTargetDescriptor {
                        label: "Alpha scene",
                        size: TargetSize::Fixed {
                            width: size[0],
                            height: size[1],
                        },
                        format: gpu.surface_format(),
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        sample_count: 1,
                    },
                );
                targets.push(scene.data.insert(owner));
            }
            let target_ids: Vec<_> = targets
                .iter()
                .map(|id| scene.data.get(*id).unwrap().id())
                .collect();
            let mut phases = Vec::with_capacity(effects + 4);
            let mut add = |scene: &mut Scene, target, phase, load| {
                let phase = scene
                    .data
                    .insert(RenderPhase::new(PhaseId(phase), gpu::SortPolicy::DrawOrder));
                scene
                    .renderer
                    .add_pass(pass::render(target, load, PhaseOperation { phase }, true));
                phases.push(phase);
            };
            let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
            add(scene, PassTarget::Offscreen(target_ids[0]), 0, clear);
            add(
                scene,
                PassTarget::Offscreen(target_ids[0]),
                1,
                wgpu::LoadOp::Load,
            );
            for target in &target_ids[1..] {
                add(scene, PassTarget::Offscreen(*target), 0, clear);
            }
            let present = if gpu.context_id().is_none() {
                let effect = velum::effects::PresentEffect::new(
                    *target_ids.last().unwrap(),
                    gpu.surface_format(),
                );
                let operation = velum::effects::PresentOperation {
                    pipeline: effect.pipeline.id(),
                    source: effect.source.id(),
                };
                scene
                    .renderer
                    .add_pass(pass::render(PassTarget::Surface, clear, operation, true));
                Some(scene.data.insert(effect))
            } else {
                add(scene, PassTarget::Surface, 0, clear);
                None
            };
            add(scene, PassTarget::Surface, 2, wgpu::LoadOp::Load);
            let text_pass = scene.renderer.add_pass(pass::render(
                PassTarget::Surface,
                wgpu::LoadOp::Load,
                super::text::HudText(text),
                false,
            ));
            *cached = Some(ScenePasses {
                present,
                text_pass,
                size,
                geometry,
                text,
                targets,
                phases,
            });
        }
        Ok((scene, cached.as_ref().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::screen::AppScreen;

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn switching_scenes_preserves_each_scenes_data_and_targets() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let gpu =
            GpuContext::from_device(device, queue, 16, 16, wgpu::TextureFormat::Rgba8UnormSrgb)
                .unwrap();
        let mut scenes = Scenes::new().unwrap();
        let (menu, passes) = scenes.prepare(&gpu, [16, 16], 0).unwrap();
        let menu_target = menu.data.require(passes.targets[0]).unwrap().id();
        let marker = menu.data.insert("menu");
        scenes.activate(&AppScreen::InGame).unwrap();
        let (game, passes) = scenes.prepare(&gpu, [32, 32], 2).unwrap();
        let game_target = game.data.require(passes.targets[0]).unwrap().id();
        assert_ne!(menu_target, game_target);
        assert_eq!(game.renderer.schedule().len(), 7);
        scenes.activate(&AppScreen::MainMenu).unwrap();
        let (menu, passes) = scenes.prepare(&gpu, [16, 16], 0).unwrap();
        assert_eq!(
            menu.data.require(passes.targets[0]).unwrap().id(),
            menu_target
        );
        let (menu, _) = scenes.prepare(&gpu, [8, 8], 1).unwrap();
        assert_eq!(*menu.data.require(marker).unwrap(), "menu");
        assert!(target::get(menu_target).is_none());
        assert!(target::get(game_target).is_some());
    }
}
