use image::RgbImage;
use std::sync::Arc;

use crate::glb::utils::GlbData;

#[derive(Clone, Debug)]
pub struct NormalMap {
    pub texture: Arc<RgbImage>,
    pub factor: f32,
}
impl NormalMap {
    pub(crate) fn load(gltf_mat: &gltf::Material, data: &mut GlbData) -> Option<Self> {
        gltf_mat.normal_texture().map(|texture| Self {
            texture: data.load_rgb_image(&texture.texture()),
            factor: texture.scale(),
        })
    }
}
