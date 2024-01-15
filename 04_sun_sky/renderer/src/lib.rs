mod renderer;
pub use renderer::Renderer;
mod scene;
pub use scene::*;

pub struct NextImage {
    pub image_view: ashtray::ImageViewHandle,
    pub sampler: ashtray::SamplerHandle,
    pub sample_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
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
    pub l_white: f32,
    pub exposure: f32,
    pub max_recursion_depth: u32,
    pub sun_direction: glam::Vec2,
    pub sun_strength: f32,
    pub sun_color: glam::Vec3,
    pub sun_angle: f32,
    pub sun_enabled: u32,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            width: 400,
            height: 300,
            max_sample_count: 256,
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            l_white: 1.0,
            exposure: 1.0,
            max_recursion_depth: 1,
            sun_direction: glam::Vec2::new(0.0, 0.0),
            sun_strength: 0.0,
            sun_color: glam::Vec3::new(0.0, 0.0, 0.0),
            sun_angle: 0.0,
            sun_enabled: 0,
        }
    }
}
