mod renderer;
pub use renderer::Renderer;

pub struct NextImage {
    pub image_view: ashtray::ImageViewHandle,
    pub sampler: ashtray::SamplerHandle,
    pub sample_count: u32,
}

#[derive(Debug, Clone)]
pub struct Parameters {
    pub width: u32,
    pub height: u32,
    pub max_sample_count: u32,
    pub rotate_x: f32,
    pub rotate_y: f32,
    pub rotate_z: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
}
