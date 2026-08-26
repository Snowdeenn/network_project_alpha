use crate::{
    app::resources::Resources,
    core::event::{DebugCollider, DebugMode, DebugRectState},
    rendering::camera::Camera,
};

pub struct DebugRenderer {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl DebugRenderer {
    pub fn init(window: &winit::window::Window, gpu_ctx: &prism::GpuContext) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            &gpu_ctx.device,
            gpu_ctx.surface_format(),
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: false,
                predictable_texture_filtering: false,
            },
        );

        Self {
            ctx,
            state,
            renderer,
        }
    }

    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn create_widget<'a>(&'a self, title: impl Into<String>) -> WidgetBuilder<'a> {
        WidgetBuilder::new_window(&self.ctx, title)
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let raw_input = self.state.take_egui_input(window);
        self.ctx.begin_pass(raw_input);
    }

    pub fn end_frame(&mut self) -> egui::FullOutput {
        self.ctx.end_pass()
    }

    pub fn flush_into(
        &mut self,
        window: &winit::window::Window,
        frame_context: &mut prism::FrameContext,
        gpu_ctx: &prism::GpuContext,
    ) {
        let mut full_output = self.end_frame();
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let paint_jobs = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&gpu_ctx.device, &gpu_ctx.queue, *id, delta);
            }
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame_context.size.0, frame_context.size.1],
            pixels_per_point: window.scale_factor() as f32,
        };

        self.renderer.update_buffers(
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &mut frame_context.encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let renderer = &mut self.renderer;
        {
            let debug_pass = frame_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Debug RenderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_context.surface_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

            let mut debug_pass = debug_pass.forget_lifetime();
            renderer.render(&mut debug_pass, &paint_jobs, &screen_descriptor);
            drop(debug_pass);
        }

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        full_output.textures_delta.clear();
    }
}

pub struct DebugUi<'a> {
    pub ui: &'a mut egui::Ui,
}

impl<'a> DebugUi<'a> {
    pub fn new(ui: &'a mut egui::Ui) -> Self {
        Self { ui }
    }

    pub fn create_widget<'b>(&'b mut self, title: impl Into<String>) -> WidgetBuilder<'b> {
        WidgetBuilder::new_child(self.ui, title)
    }

    pub fn button(&mut self, text: impl Into<String>) -> egui::Response {
        self.ui.button(text.into())
    }

    pub fn toggle(&mut self, label: &str, value: &mut bool) -> egui::Response {
        self.ui.checkbox(value, label)
    }

    pub fn checkbox(&mut self, label: &str) -> bool {
        let id = self.ui.make_persistent_id(label);
        let mut state = self
            .ui
            .ctx()
            .data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));
        self.ui.checkbox(&mut state, label);
        self.ui.ctx().data_mut(|d| d.insert_temp(id, state));
        state
    }

    pub fn slider(
        &mut self,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) -> egui::Response {
        self.ui.add(egui::Slider::new(value, range).text(label))
    }

    pub fn metric(&mut self, label: &str, value: impl std::fmt::Display) {
        self.ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{label} :")).strong());
            ui.label(value.to_string());
        });
    }

    pub fn metric_colored(
        &mut self,
        label: &str,
        value: impl std::fmt::Display,
        color: egui::Color32,
    ) {
        self.ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{label} :")).strong());
            ui.colored_label(color, value.to_string());
        });
    }
}

pub enum WidgetKind {
    Window,
    Header { default_open: bool },
    Group,
    Horizontal,
    Vertical,
}

pub struct WidgetBuilder<'a> {
    ctx: Option<&'a egui::Context>,
    parent_ui: Option<&'a mut egui::Ui>,
    title: String,
    kind: WidgetKind,
    size: Option<[f32; 2]>,
    resizable: bool,
    collapsible: bool,
}

impl<'a> WidgetBuilder<'a> {
    pub fn new_window(ctx: &'a egui::Context, title: impl Into<String>) -> Self {
        Self {
            ctx: Some(ctx),
            parent_ui: None,
            title: title.into(),
            kind: WidgetKind::Window,
            size: None,
            resizable: true,
            collapsible: true,
        }
    }

    pub fn new_child(parent_ui: &'a mut egui::Ui, title: impl Into<String>) -> Self {
        Self {
            ctx: None,
            parent_ui: Some(parent_ui),
            title: title.into(),
            kind: WidgetKind::Group,
            size: None,
            resizable: false,
            collapsible: false,
        }
    }

    // --- Méthodes du Builder ---

    pub fn kind(mut self, kind: WidgetKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn window(mut self) -> Self {
        self.kind = WidgetKind::Window;
        self
    }

    pub fn header(mut self, default_open: bool) -> Self {
        self.kind = WidgetKind::Header { default_open };
        self
    }

    pub fn group(mut self) -> Self {
        self.kind = WidgetKind::Group;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.kind = WidgetKind::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.kind = WidgetKind::Vertical;
        self
    }

    pub fn size(mut self, size: [f32; 2]) -> Self {
        self.size = Some(size);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    pub fn show<R>(self, add_contents: impl FnOnce(&mut DebugUi) -> R) -> Option<R> {
        match self.kind {
            WidgetKind::Window => {
                let ctx = self.ctx?;
                let mut win = egui::Window::new(&self.title)
                    .resizable(self.resizable)
                    .collapsible(self.collapsible);

                if let Some(s) = self.size {
                    win = win.default_size(s);
                }

                let mut result = None;
                win.show(ctx, |ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Header { default_open } => {
                let ui = self.parent_ui?;
                let mut result = None;
                egui::CollapsingHeader::new(egui::RichText::new(&self.title).strong())
                    .default_open(default_open)
                    .show(ui, |ui| {
                        let mut widget = DebugUi::new(ui);
                        result = Some(add_contents(&mut widget));
                    });
                ui.separator();
                result
            }
            WidgetKind::Group => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.group(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Horizontal => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.horizontal(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
            WidgetKind::Vertical => {
                let ui = self.parent_ui?;
                let mut result = None;
                ui.vertical(|ui| {
                    let mut widget = DebugUi::new(ui);
                    result = Some(add_contents(&mut widget));
                });
                result
            }
        }
    }
}

#[derive(Default)]
pub struct DebugData {
    data: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

impl DebugData {
    pub fn insert<T: 'static>(&mut self, value: T) {
        self.data
            .insert(std::any::TypeId::of::<T>(), Box::new(value));
    }
    pub fn read<T: 'static>(&self) -> Option<&T> {
        self.data
            .get(&std::any::TypeId::of::<T>())
            .and_then(|d| d.downcast_ref::<T>())
    }
    pub fn update_data<T: 'static>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|d| d.downcast_mut::<T>())
    }
}

pub fn run_debug(
    debug_renderer: &DebugRenderer,
    mode: DebugMode,
    debug_data: &DebugData,
    resources: &Resources,
) {
    let attack_box = debug_data.read::<Vec<DebugRectState>>().unwrap();
    let colliders = debug_data.read::<Vec<DebugCollider>>().unwrap();
    if mode == DebugMode::Off {
        return;
    }

    if mode == DebugMode::Interactive {
        let painter = debug_renderer.ctx.layer_painter(egui::LayerId {
            order: egui::Order::Background,
            id: egui::Id::new("debug_overlay"),
        });
        debug_renderer
            .create_widget("🛠️ Panneau de Contrôle Debug")
            .window()
            .size([360.0, 500.0])
            .show(|w| {
                w.metric_colored("Mode actif", format!("{:?}", mode), egui::Color32::GREEN);

                // Section Combat
                w.create_widget("Combat & Collisions")
                    .header(true)
                    .show(|w| {
                        w.metric("Attack boxes", attack_box.len());
                        w.metric("Colliders", colliders.len());
                    });

                w.create_widget("CostField & DirectionField")
                    .header(true)
                    .show(|w| {
                        if w.checkbox("Afficher cost field") {
                            draw_cost_field(&painter, debug_data, resources);
                        }
                        if w.checkbox("Afficher direction field") {
                            draw_direction_field(&painter, debug_data, resources);
                        }
                    })
            });
    }
}

fn draw_cost_field(painter: &egui::Painter, debug_data: &DebugData, resources: &Resources) {
    let grid = resources.read_resource::<utils::map::grid::Grid>();
    let cam = debug_data.read::<Camera>().unwrap();
    let screen_size = debug_data.read::<winit::dpi::PhysicalSize<u32>>().unwrap();

    if let Some(flow_field) = debug_data.read::<utils::map::flow_field::FlowField>() {
        for (x, y, _cell) in grid.iter_with_pos() {
            let world = grid.grid_to_world(x, y);
            let screen_x = (world.x - cam.pos().x) + screen_size.width as f32 / 2.0;
            let screen_y = (world.y - cam.pos().y) + screen_size.height as f32 / 2.0;
            let center = egui::pos2(screen_x, screen_y);
            let tile = grid.cell_size();
            let cell_index = grid.cell_index(x, y);
            let cost = flow_field.cost_field()[cell_index];

            let intensity = (cost as f32 / 120.0).min(1.0);
            let r = (intensity * 255.0) as u8;
            let g = ((1.0 - intensity) * 255.0) as u8;
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::vec2(tile - 2.0, tile - 2.0)),
                0.0,
                egui::Color32::from_rgba_unmultiplied(r, g, 50, 90),
            );
        }
    }
}

fn draw_direction_field(painter: &egui::Painter, debug_data: &DebugData, resources: &Resources) {
    let grid = resources.read_resource::<utils::map::grid::Grid>();
    let cam = debug_data.read::<Camera>().unwrap();
    let screen_size = debug_data.read::<winit::dpi::PhysicalSize<u32>>().unwrap();

    if let Some(flow_field) = debug_data.read::<utils::map::flow_field::FlowField>() {
        for (x, y, _cell) in grid.iter_with_pos() {
            let world = grid.grid_to_world(x, y);
            let screen_x = (world.x - cam.pos().x) + screen_size.width as f32 / 2.0;
            let screen_y = (world.y - cam.pos().y) + screen_size.height as f32 / 2.0;
            let cell_index = grid.cell_index(x, y);
            let dir = flow_field.direction_field()[cell_index];
            let arrow_len = 20.0;
            painter.line_segment(
                [
                    egui::pos2(screen_x, screen_y),
                    egui::pos2(screen_x + dir.x * arrow_len, screen_y + dir.y * arrow_len),
                ],
                egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(0, 200, 255, 200)),
            );
        }
    }
}
