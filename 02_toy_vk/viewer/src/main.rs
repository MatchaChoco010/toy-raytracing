use ash::{extensions::khr::Swapchain, vk};
use ashtray::{
    utils, CommandPoolHandle, DeviceHandle, FenceHandle, InstanceHandle, SemaphoreHandle,
    SurfaceHandle,
};
use egui_ash::{
    raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle},
    App, AppCreator, AshRenderState, CreationContext, HandleRedraw, RunOption,
};
use gpu_allocator::vulkan::*;
use std::sync::{Arc, Mutex};

mod scene_view;

struct ViewerInner {
    width: u32,
    height: u32,
    surface: SurfaceHandle,
    physical_device: vk::PhysicalDevice,
    device: DeviceHandle,
    queue_handles: ashtray::utils::QueueHandles,
    swapchain: Option<ashtray::utils::SwapchainObjects>,
    command_pool: CommandPoolHandle,
    command_buffers: Vec<ashtray::CommandBufferHandle>,
    image_available_semaphores: Vec<SemaphoreHandle>,
    render_finished_semaphores: Vec<SemaphoreHandle>,
    in_flight_fences: Vec<FenceHandle>,
    current_frame: usize,
    dirty_swapchain: bool,
}

struct Viewer {
    inner: Arc<Mutex<ViewerInner>>,
    scene_view: scene_view::SceneView,
}
impl Viewer {
    fn new(
        width: u32,
        height: u32,
        surface: SurfaceHandle,
        physical_device: vk::PhysicalDevice,
        device: DeviceHandle,
        queue_handles: ashtray::utils::QueueHandles,
        command_pool: CommandPoolHandle,
        scene_view: scene_view::SceneView,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ViewerInner {
                width,
                height,
                surface,
                physical_device,
                device,
                queue_handles,
                swapchain: None,
                command_pool,
                command_buffers: vec![],
                image_available_semaphores: vec![],
                render_finished_semaphores: vec![],
                in_flight_fences: vec![],
                current_frame: 0,
                dirty_swapchain: true,
            })),
            scene_view,
        }
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
        });
    }

    fn request_redraw(&mut self, _viewport_id: egui::ViewportId) -> HandleRedraw {
        self.scene_view.redraw();
        HandleRedraw::Handle({
            let inner = self.inner.clone();
            Box::new(
                move |size: egui_ash::winit::dpi::PhysicalSize<u32>, mut egui_cmd| {
                    let mut inner = inner.lock().unwrap();

                    // recreate swapchain
                    if inner.dirty_swapchain
                        || inner.width != size.width
                        || inner.height != size.height
                    {
                        inner.device.wait_idle();

                        {
                            // 破棄
                            inner.swapchain.take();
                            inner.command_buffers.clear();
                            inner.image_available_semaphores.clear();
                            inner.render_finished_semaphores.clear();
                            inner.in_flight_fences.clear();
                        }

                        inner.width = size.width;
                        inner.height = size.height;
                        let swapchain = ashtray::utils::create_swapchain_objects(
                            size.width,
                            size.height,
                            &inner.surface.clone(),
                            inner.physical_device,
                            &inner.device.clone(),
                        );

                        let command_buffers = ashtray::utils::allocate_command_buffers(
                            &swapchain.swapchain.device(),
                            &inner.command_pool,
                            swapchain.swapchain_images.len() as u32,
                        );

                        let image_available_semaphores = (0..swapchain.swapchain_images.len())
                            .map(|_| {
                                inner
                                    .device
                                    .create_semaphore(&vk::SemaphoreCreateInfo::default())
                            })
                            .collect::<Vec<_>>();
                        let render_finished_semaphores = (0..swapchain.swapchain_images.len())
                            .map(|_| {
                                inner
                                    .device
                                    .create_semaphore(&vk::SemaphoreCreateInfo::default())
                            })
                            .collect::<Vec<_>>();
                        let in_flight_fences = (0..swapchain.swapchain_images.len())
                            .map(|_| ashtray::utils::create_signaled_fence(&inner.device))
                            .collect::<Vec<_>>();

                        let swapchain_info = egui_ash::SwapchainUpdateInfo {
                            width: size.width,
                            height: size.height,
                            swapchain_images: swapchain.swapchain_images.clone(),
                            surface_format: swapchain.format,
                        };
                        egui_cmd.update_swapchain(swapchain_info);

                        inner.swapchain = Some(swapchain);
                        inner.command_buffers = command_buffers;
                        inner.image_available_semaphores = image_available_semaphores;
                        inner.render_finished_semaphores = render_finished_semaphores;
                        inner.in_flight_fences = in_flight_fences;
                    }

                    let swapchain = &inner.swapchain.as_ref().unwrap().swapchain;
                    let swapchain_images = &inner.swapchain.as_ref().unwrap().swapchain_images;

                    // acquire next image
                    let result = inner.device.acquire_next_image(
                        swapchain,
                        u64::MAX,
                        Some(inner.image_available_semaphores[inner.current_frame].clone()),
                        None,
                    );
                    let index = match result {
                        Ok((index, _)) => index as usize,
                        Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                            // recreate swapchain
                            inner.dirty_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                    // wait for fence
                    inner.device.wait_fences(
                        &[inner.in_flight_fences[inner.current_frame].clone()],
                        u64::MAX,
                    );
                    inner
                        .device
                        .reset_fences(&[inner.in_flight_fences[inner.current_frame].clone()]);

                    // record command buffers
                    let command_buffer = inner.command_buffers[index].clone();
                    command_buffer.begin_command_buffer(
                        &vk::CommandBufferBeginInfo::builder()
                            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                    );
                    ashtray::utils::cmd_image_barriers(
                        &command_buffer,
                        vk::PipelineStageFlags2::TOP_OF_PIPE,
                        vk::AccessFlags2::NONE,
                        vk::ImageLayout::UNDEFINED,
                        vk::PipelineStageFlags2::CLEAR,
                        vk::AccessFlags2::TRANSFER_WRITE,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &swapchain_images[index],
                    );

                    command_buffer.cmd_clear_color_image(
                        &swapchain_images[index],
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                        &[vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1)
                            .build()],
                    );
                    ashtray::utils::cmd_image_barriers(
                        &command_buffer,
                        vk::PipelineStageFlags2::CLEAR,
                        vk::AccessFlags2::TRANSFER_WRITE,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::PipelineStageFlags2::FRAGMENT_SHADER,
                        vk::AccessFlags2::SHADER_READ,
                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        &swapchain_images[index],
                    );
                    egui_cmd.record(unsafe { command_buffer.command_buffer_raw() }, index);
                    command_buffer.end_command_buffer();

                    // submit command buffers
                    let buffers_to_submit = [*command_buffer];
                    let submit_info = vk::SubmitInfo::builder()
                        .command_buffers(&buffers_to_submit)
                        .wait_semaphores(std::slice::from_ref(
                            &inner.image_available_semaphores[inner.current_frame],
                        ))
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .signal_semaphores(std::slice::from_ref(
                            &inner.render_finished_semaphores[inner.current_frame],
                        ));
                    inner.device.queue_submit(
                        inner.queue_handles.graphics.queue,
                        std::slice::from_ref(&submit_info),
                        Some(inner.in_flight_fences[inner.current_frame].clone()),
                    );

                    // present image
                    let image_indices = [index as u32];
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(std::slice::from_ref(
                            &inner.render_finished_semaphores[inner.current_frame],
                        ))
                        .swapchains(std::slice::from_ref(swapchain))
                        .image_indices(&image_indices);
                    let result = inner
                        .device
                        .queue_present(inner.queue_handles.present.queue, &present_info);
                    let is_dirty_swapchain = match result {
                        Ok(true)
                        | Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                            true
                        }
                        Err(error) => panic!("Failed to present queue. Cause: {}", error),
                        _ => false,
                    };
                    inner.dirty_swapchain = is_dirty_swapchain;

                    // update current frame
                    inner.current_frame = (inner.current_frame + 1) % inner.in_flight_fences.len();
                },
            )
        })
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
        let app = Viewer::new(
            cc.main_window.inner_size().width,
            cc.main_window.inner_size().height,
            surface.clone(),
            physical_device,
            device.clone(),
            queue_handles.clone(),
            command_pool.clone(),
            scene_view,
        );

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
            ..Default::default()
        },
    )
}
