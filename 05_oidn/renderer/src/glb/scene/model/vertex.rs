use glam::{Vec2, Vec3, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub tex_coords: Vec2,
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            position: Vec3::ZERO,
            normal: Vec3::Z,
            tangent: Vec4::X,
            tex_coords: Vec2::ZERO,
        }
    }
}
