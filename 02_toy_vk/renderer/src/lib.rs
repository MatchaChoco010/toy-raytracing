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
}
