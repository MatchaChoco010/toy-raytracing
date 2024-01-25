use std::time::Duration;

mod renderer;
pub use renderer::Renderer;
mod scene;
pub use scene::*;

pub struct NextImage {
    pub image_view: ashtray::ImageViewHandle,
    pub sampler: ashtray::SamplerHandle,
    pub sample_count: u32,
    pub rendering_time: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayImage {
    BaseColor,
    Normal,
    Resolved,
    Final,
}

#[derive(Debug, Clone)]
pub struct Parameters {
    pub width: u32,
    pub height: u32,
    pub max_sample_count: u32,
    pub display_image: DisplayImage,
    pub rotate_x: f32,
    pub rotate_y: f32,
    pub rotate_z: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub fov: f32,
    pub l_white: f32,
    pub aperture: f32,
    pub shutter_speed: f32,
    pub iso: f32,
    pub max_recursion_depth: u32,
    pub sun_direction: glam::Vec2,
    pub sun_strength: f32,
    pub sun_color: glam::Vec3,
    pub sun_angle: f32,
    pub sun_enabled: u32,
    pub sky_rotation: f32,
    pub sky_strength: f32,
    pub sky_enabled: u32,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            width: 400,
            height: 300,
            max_sample_count: 256,
            display_image: DisplayImage::Final,
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            fov: 60.0_f32.to_radians(),
            l_white: 1.0,
            aperture: 16.0,
            shutter_speed: 1.0 / 100.0,
            iso: 100.0,
            max_recursion_depth: 1,
            sun_direction: glam::Vec2::new(0.0, 0.0),
            sun_strength: 0.0,
            sun_color: glam::Vec3::new(0.0, 0.0, 0.0),
            sun_angle: 0.0,
            sun_enabled: 0,
            sky_rotation: 0.0,
            sky_strength: 0.0,
            sky_enabled: 0,
        }
    }
}
impl PartialEq for Parameters {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.max_sample_count == other.max_sample_count
            // && self.display_image == other.display_image
            && self.rotate_x == other.rotate_x
            && self.rotate_y == other.rotate_y
            && self.rotate_z == other.rotate_z
            && self.position_x == other.position_x
            && self.position_y == other.position_y
            && self.position_z == other.position_z
            && self.fov == other.fov
            && self.l_white == other.l_white
            && self.aperture == other.aperture
            && self.shutter_speed == other.shutter_speed
            && self.iso == other.iso
            && self.max_recursion_depth == other.max_recursion_depth
            && self.sun_direction == other.sun_direction
            && self.sun_strength == other.sun_strength
            && self.sun_color == other.sun_color
            && self.sun_angle == other.sun_angle
            && self.sun_enabled == other.sun_enabled
            && self.sky_rotation == other.sky_rotation
            && self.sky_strength == other.sky_strength
            && self.sky_enabled == other.sky_enabled
    }
}
