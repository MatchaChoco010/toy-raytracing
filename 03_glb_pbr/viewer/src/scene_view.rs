use ashtray::{ImageViewHandle, SamplerHandle};
use std::sync::{Arc, Mutex};

pub struct SceneViewState {
    pub fit_view: bool,
    pub width: u32,
    pub height: u32,
    pub max_sample_count: u32,
    pub sample_count: u32,
    pub rotate_x: f32,
    pub rotate_y: f32,
    pub rotate_z: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub l_white: f32,
    pub max_recursion_depth: u32,
}

struct SceneViewInner {
    renderer: renderer::Renderer,

    image_registry: egui_ash::ImageRegistry,
    scene_image: Option<egui::TextureId>,

    current_image_view: Option<ImageViewHandle>,
    current_sampler: Option<SamplerHandle>,

    pub state: Arc<Mutex<SceneViewState>>,
}

#[derive(Clone)]
pub struct SceneView {
    inner: Arc<Mutex<SceneViewInner>>,
}
impl SceneView {
    pub fn new(renderer: renderer::Renderer, image_registry: egui_ash::ImageRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SceneViewInner {
                renderer,

                image_registry,
                scene_image: None,

                current_image_view: None,
                current_sampler: None,

                state: Arc::new(Mutex::new(SceneViewState {
                    fit_view: false,
                    width: 400,
                    height: 300,
                    max_sample_count: 256,
                    sample_count: 0,
                    rotate_x: 0.0,
                    rotate_y: 0.0,
                    rotate_z: 0.0,
                    position_x: 0.0,
                    position_y: 2.0,
                    position_z: 5.0,
                    l_white: 1.0,
                    max_recursion_depth: 8,
                })),
            })),
        }
    }

    pub fn redraw(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        let state = inner.state.clone();
        let mut state = state.lock().unwrap();
        let next_image = inner.renderer.render(renderer::Parameters {
            width: state.width,
            height: state.height,
            max_sample_count: state.max_sample_count,
            rotate_x: state.rotate_x,
            rotate_y: state.rotate_y,
            rotate_z: state.rotate_z,
            position_x: state.position_x,
            position_y: state.position_y,
            position_z: state.position_z,
            l_white: state.l_white,
            max_recursion_depth: state.max_recursion_depth,
        });
        let texture_id = unsafe {
            inner.image_registry.register_user_texture(
                next_image.image_view.image_view_raw(),
                next_image.sampler.sampler_raw(),
            )
        };

        inner.current_image_view = Some(next_image.image_view);
        inner.current_sampler = Some(next_image.sampler);
        state.sample_count = next_image.sample_count;

        if let Some(texture_id) = inner.scene_image.take() {
            inner.image_registry.unregister_user_texture(texture_id);
        }
        inner.scene_image = Some(texture_id);
    }

    pub fn state(&self) -> Arc<Mutex<SceneViewState>> {
        self.inner.lock().unwrap().state.clone()
    }
}
impl egui::Widget for &mut SceneView {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let inner = self.inner.lock().unwrap();
        let mut state = inner.state.lock().unwrap();

        let mut size = None;
        let response = if let Some(texture_id) = inner.scene_image {
            if state.fit_view {
                let image_size = ui.available_size();
                size = Some(image_size);
                ui.image(egui::load::SizedTexture {
                    id: texture_id,
                    size: image_size,
                })
            } else {
                // layoutとdrag areaを組み合わせると謎にx方向にズレが生じるので対策
                let ui_size = ui.available_size();
                if ui_size.x < state.width as f32 {
                    egui::scroll_area::ScrollArea::both()
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::top_down(egui::Align::LEFT)
                                    .with_main_align(egui::Align::Center)
                                    .with_main_justify(true)
                                    .with_cross_justify(true),
                                |ui| {
                                    ui.image(egui::load::SizedTexture {
                                        id: texture_id,
                                        size: egui::Vec2::new(
                                            state.width as f32,
                                            state.height as f32,
                                        ),
                                    })
                                },
                            )
                            .inner
                        })
                        .inner
                } else {
                    egui::scroll_area::ScrollArea::both()
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::top_down(egui::Align::Center)
                                    .with_main_align(egui::Align::Center)
                                    .with_main_justify(true)
                                    .with_cross_justify(true),
                                |ui| {
                                    ui.image(egui::load::SizedTexture {
                                        id: texture_id,
                                        size: egui::Vec2::new(
                                            state.width as f32,
                                            state.height as f32,
                                        ),
                                    })
                                },
                            )
                            .inner
                        })
                        .inner
                }
            }
        } else {
            ui.add(egui::widgets::Spinner::default())
        };
        let rect = egui::Rect::shrink(response.rect, 20.0);
        let response = response.with_new_rect(rect);
        let response = response.interact(egui::Sense::drag());

        if response.dragged_by(egui::PointerButton::Primary) {
            state.rotate_y =
                (state.rotate_y + response.drag_delta().x / 5.0 + 180.0) % 360.0 - 180.0;
            state.rotate_x = (state.rotate_x + response.drag_delta().y / 5.0).clamp(-90.0, 90.0);
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            let position = glam::vec3(state.position_x, state.position_y, state.position_z);
            let direction_x = glam::Mat4::from_euler(
                glam::EulerRot::YXZ,
                state.rotate_y.to_radians(),
                state.position_x.to_radians(),
                state.position_z.to_radians(),
            )
            .transform_vector3(glam::Vec3::NEG_X);
            let direction_y = glam::Mat4::from_euler(
                glam::EulerRot::YXZ,
                state.rotate_y.to_radians(),
                state.position_x.to_radians(),
                state.position_z.to_radians(),
            )
            .transform_vector3(glam::Vec3::Y);

            let position = position
                + direction_x * response.drag_delta().x / 100.0
                + direction_y * response.drag_delta().y / 100.0;
            state.position_x = position.x;
            state.position_y = position.y;
            state.position_z = position.z;
        }
        let scroll_delta = ui.input(|i| i.scroll_delta);
        if scroll_delta.y != 0.0 && response.hovered() {
            let position = glam::vec3(state.position_x, state.position_y, state.position_z);
            let direction = glam::Mat4::from_euler(
                glam::EulerRot::YXZ,
                state.rotate_y.to_radians(),
                state.position_x.to_radians(),
                state.position_z.to_radians(),
            )
            .transform_vector3(glam::Vec3::NEG_Z);
            let position = position + direction * scroll_delta.y / 100.0;
            state.position_x = position.x;
            state.position_y = position.y;
            state.position_z = position.z;
        }

        // update state
        {
            if let Some(size) = size {
                if state.width != size.x as u32 || state.height != size.y as u32 {
                    state.width = size.x as u32;
                    state.height = size.y as u32;
                }
            }
        }

        response
    }
}
