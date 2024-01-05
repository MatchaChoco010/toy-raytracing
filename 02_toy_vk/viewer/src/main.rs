use ash::{extensions::khr::Swapchain, vk};
use ashtray::{utils, InstanceHandle};
use egui_ash::{
    raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle},
    App, AppCreator, AshRenderState, CreationContext, HandleRedraw, RunOption,
};
use gpu_allocator::vulkan::*;
use std::sync::{Arc, Mutex};

mod scene_view;

struct Viewer {
    scene_view: scene_view::SceneView,
}
impl Viewer {
    fn new(scene_view: scene_view::SceneView) -> Self {
        Self { scene_view }
    }
}
impl App for Viewer {
    fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Scene View").show(ctx, |ui| {
            ui.add(&mut self.scene_view);
        });
        egui::Window::new("Stats").show(ctx, |ui| {
            ui.label(format!("sample_count: {}", self.scene_view.sample_count));
            ui.label(format!(
                "size: {}x{}",
                self.scene_view.width, self.scene_view.height
            ));
        });
        egui::Window::new("Parameters").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("max_sample_count: ");
                ui.add(egui::widgets::DragValue::new(
                    &mut self.scene_view.max_sample_count,
                ));
            });
            ui.add(egui::widgets::Checkbox::new(
                &mut self.scene_view.fit_view,
                "fit view",
            ));
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("size: ");
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.width));
                ui.label("x");
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.height));
            });

            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("camera position: ");
                ui.add(egui::widgets::DragValue::new(
                    &mut self.scene_view.position_x,
                ));
                ui.add(egui::widgets::DragValue::new(
                    &mut self.scene_view.position_y,
                ));
                ui.add(egui::widgets::DragValue::new(
                    &mut self.scene_view.position_z,
                ));
            });
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("camera rotate: ");
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.rotate_x));
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.rotate_y));
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.rotate_z));
            });
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("L_white: ");
                ui.add(egui::widgets::DragValue::new(&mut self.scene_view.l_white));
            });
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("max recursion depth: ");
                ui.add(egui::widgets::DragValue::new(
                    &mut self.scene_view.max_recursion_depth,
                ));
            });
        });
    }

    fn request_redraw(&mut self, _viewport_id: egui::ViewportId) -> HandleRedraw {
        self.scene_view.redraw();
        HandleRedraw::Auto
    }
}

struct ViewerCreator;
impl AppCreator<Arc<Mutex<Allocator>>> for ViewerCreator {
    type App = Viewer;

    fn create(&self, cc: CreationContext) -> (Self::App, AshRenderState<Arc<Mutex<Allocator>>>) {
        // create vulkan stuffs
        let instance = InstanceHandle::new(cc.main_window.raw_display_handle());
        let surface = instance.create_surface(
            cc.main_window.raw_display_handle(),
            cc.main_window.raw_window_handle(),
        );
        let required_device_extensions =
            utils::get_required_device_extensions(&cc.required_device_extensions);
        let physical_device =
            utils::select_physical_device(&instance, &surface, &required_device_extensions);
        let queue_indices = utils::get_queue_indices(&instance, &surface, physical_device);
        let device = utils::create_device(
            &instance,
            physical_device,
            &queue_indices,
            &required_device_extensions,
        );
        let swapchain_loader = Swapchain::new(&instance, &device);
        let queue_handles = utils::get_queue_handles(&device, &queue_indices);
        let command_pool = utils::create_graphics_command_pool(&device, &queue_handles);
        let allocator = utils::create_allocator(&instance, physical_device, &device);

        // create renderer
        let mut renderer = renderer::Renderer::new(
            800,
            600,
            instance.clone(),
            physical_device,
            device.clone(),
            queue_handles.clone(),
            command_pool.clone(),
            allocator.clone(),
        );

        // load scene
        let scene = renderer::Scene {
            materials: vec![
                renderer::Material {
                    color: glam::vec3(1.0, 1.0, 1.0),
                    ty: 0,
                },
                renderer::Material {
                    color: glam::vec3(1.0, 0.0, 0.0),
                    ty: 0,
                },
                renderer::Material {
                    color: glam::vec3(0.0, 1.0, 0.0),
                    ty: 0,
                },
                renderer::Material {
                    color: glam::vec3(10.0, 10.0, 10.0),
                    ty: 1,
                },
                renderer::Material {
                    color: glam::vec3(1.0, 1.0, 1.0),
                    ty: 2,
                },
            ],
            meshes: vec![
                renderer::Mesh {
                    path: "assets/bunny.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/box.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/hidari.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/migi.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/oku.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/tenjou.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/yuka.obj".into(),
                },
                renderer::Mesh {
                    path: "assets/light.obj".into(),
                },
            ],
            instances: vec![
                renderer::Instance {
                    mesh_index: 0,
                    material_index: 4,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 1,
                    material_index: 0,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 2,
                    material_index: 1,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 3,
                    material_index: 2,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 4,
                    material_index: 0,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 5,
                    material_index: 0,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 6,
                    material_index: 0,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    mesh_index: 7,
                    material_index: 3,
                    transform: glam::Mat4::IDENTITY,
                },
            ],
        };
        renderer.load_scene(&scene);

        // create scene view
        let scene_view = scene_view::SceneView::new(renderer, cc.image_registry);

        // create app
        let app = Viewer::new(scene_view);

        // create ash render state
        let ash_render_state = AshRenderState {
            entry: unsafe { instance.entry_raw() },
            instance: unsafe { instance.instance_raw() },
            physical_device,
            device: unsafe { device.device_raw() },
            surface_loader: unsafe { surface.surface_loader_raw() },
            swapchain_loader,
            queue: queue_handles.graphics.queue,
            queue_family_index: queue_handles.graphics.family_index,
            command_pool: unsafe { command_pool.command_pool_raw() },
            allocator: unsafe { allocator.allocator_raw() },
        };

        (app, ash_render_state)
    }
}

#[tokio::main]
async fn main() {
    egui_ash::run(
        "02_toy_vk-viewer",
        ViewerCreator,
        RunOption {
            viewport_builder: Some(
                egui::ViewportBuilder::default()
                    .with_title("02_toy_vk-viewer")
                    .with_inner_size(egui::vec2(1200.0, 800.0)),
            ),
            present_mode: vk::PresentModeKHR::MAILBOX,
            ..Default::default()
        },
    )
}
