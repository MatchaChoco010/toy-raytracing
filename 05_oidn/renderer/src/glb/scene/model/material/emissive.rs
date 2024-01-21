use glam::Vec3;
use image::RgbImage;
use std::sync::Arc;

use crate::glb::utils::GlbData;

#[derive(Clone, Debug)]
pub struct Emissive {
    pub texture: Option<Arc<RgbImage>>,
    pub factor: Vec3,
}

impl Emissive {
    pub(crate) fn load(gltf_mat: &gltf::Material, data: &mut GlbData) -> Self {
        Self {
            texture: gltf_mat
                .emissive_texture()
                .map(|texture| data.load_rgb_image(&texture.texture())),
            factor: gltf_mat.emissive_factor().into(),
        }
    }
}

impl Default for Emissive {
    fn default() -> Self {
        Self {
            texture: None,
            factor: Vec3::ZERO,
        }
    }
}
