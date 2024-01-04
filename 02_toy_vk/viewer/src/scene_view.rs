use ashtray::{ImageViewHandle, SamplerHandle};

pub struct SceneView {
    renderer: renderer::Renderer,

    image_registry: egui_ash::ImageRegistry,
    scene_image: Option<egui::TextureId>,

    width: u32,
    height: u32,
    current_image_view: Option<ImageViewHandle>,
    current_sampler: Option<SamplerHandle>,
    pub max_sample_count: u32,
    pub sample_count: u32,
}
impl SceneView {
    pub fn new(renderer: renderer::Renderer, image_registry: egui_ash::ImageRegistry) -> Self {
        Self {
            renderer,

            image_registry,
            scene_image: None,

            width: 800,
            height: 600,
            current_image_view: None,
            current_sampler: None,
            max_sample_count: 256,
            sample_count: 0,
        }
    }

    pub fn redraw(&mut self) {
        let next_image = self.renderer.render(renderer::Parameters {
            width: self.width,
            height: self.height,
            max_sample_count: self.max_sample_count,
        });
        let texture_id = unsafe {
            self.image_registry.register_user_texture(
                next_image.image_view.image_view_raw(),
                next_image.sampler.sampler_raw(),
            )
        };

        self.current_image_view = Some(next_image.image_view);
        self.current_sampler = Some(next_image.sampler);
        self.sample_count = next_image.sample_count;

        if let Some(texture_id) = self.scene_image.take() {
            self.image_registry.unregister_user_texture(texture_id);
        }
        self.scene_image = Some(texture_id);
    }
}
impl egui::Widget for &mut SceneView {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut size = None;
        let response = ui
            .with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    let image_size = ui.available_size();
                    size = Some(image_size);
                    if let Some(texture_id) = self.scene_image {
                        ui.image(egui::load::SizedTexture {
                            id: texture_id,
                            size: image_size,
                        })
                    } else {
                        ui.add(egui::widgets::Spinner::default())
                    }
                },
            )
            .response;
        let response = response.interact(egui::Sense::drag());
        // if response.dragged() {
        //     inner.rotate_y = (inner.rotate_y - response.drag_delta().x + 180.0) % 360.0 - 180.0;
        //     inner.rotate_x = (inner.rotate_x - response.drag_delta().y).clamp(-90.0, 90.0);
        // }

        // update state
        {
            if let Some(size) = size {
                if self.width != size.x as u32 || self.height != size.y as u32 {
                    self.width = size.x as u32;
                    self.height = size.y as u32;
                }
            }
        }

        response
    }
}
