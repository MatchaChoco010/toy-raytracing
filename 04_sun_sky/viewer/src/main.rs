use ash::{extensions::khr::Swapchain, vk};
use ashtray::{utils, InstanceHandle};
use egui_ash::{
    raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle},
    App, AppCreator, AshRenderState, CreationContext, HandleRedraw, RunOption,
};
use gpu_allocator::vulkan::*;
use std::sync::{Arc, Mutex};

mod pane;
mod scene_view;
mod tree_behaviour;

struct Viewer {
    scene_view: scene_view::SceneView,
    tree: egui_tiles::Tree<pane::Pane>,
    tree_behavior: tree_behaviour::TreeBehavior,
}
impl Viewer {
    fn new(scene_view: scene_view::SceneView) -> Self {
        let tree = pane::Pane::create_tree(scene_view.clone());
        Self {
            scene_view,
            tree,
            tree_behavior: tree_behaviour::TreeBehavior,
        }
    }
}
impl App for Viewer {
    fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let behavior = &mut self.tree_behavior;
            self.tree.ui(behavior, ui);
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
            glbs: vec![
                renderer::Glb {
                    path: "assets/glb/dragon-1.glb".into(),
                },
                renderer::Glb {
                    path: "assets/glb/dragon-2.glb".into(),
                },
                renderer::Glb {
                    path: "assets/glb/dragon-3.glb".into(),
                },
                renderer::Glb {
                    path: "assets/glb/plane.glb".into(),
                },
                renderer::Glb {
                    path: "assets/glb/light.glb".into(),
                },
            ],
            instances: vec![
                renderer::Instance {
                    glb_index: 0,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    glb_index: 1,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    glb_index: 2,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    glb_index: 3,
                    transform: glam::Mat4::IDENTITY,
                },
                renderer::Instance {
                    glb_index: 4,
                    transform: glam::Mat4::from_translation(glam::vec3(-2.0, 0.0, 0.0)),
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
            allocator: allocator.allocator(),
        };

        (app, ash_render_state)
    }
}

fn main() {
    egui_ash::run(
        "04_sun_sky-viewer",
        ViewerCreator,
        RunOption {
            viewport_builder: Some(
                egui::ViewportBuilder::default()
                    .with_title("04_sun_sky")
                    .with_inner_size(egui::vec2(1400.0, 800.0)),
            ),
            present_mode: vk::PresentModeKHR::MAILBOX,
            follow_system_theme: true,
            default_theme: egui_ash::Theme::Dark,
            ..Default::default()
        },
    );
}
