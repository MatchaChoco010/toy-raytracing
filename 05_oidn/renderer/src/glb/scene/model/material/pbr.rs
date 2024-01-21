use glam::Vec4;
use image::{GrayImage, RgbaImage};
use std::sync::Arc;

use crate::glb::utils::GlbData;

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub base_color_factor: Vec4,
    pub base_color_texture: Option<Arc<RgbaImage>>,
    pub metallic_texture: Option<Arc<GrayImage>>,
    pub metallic_factor: f32,
    pub roughness_texture: Option<Arc<GrayImage>>,
    pub roughness_factor: f32,
}
impl PbrMaterial {
    pub(crate) fn load(pbr: gltf::material::PbrMetallicRoughness, data: &mut GlbData) -> Self {
        let mut material = Self {
            base_color_factor: pbr.base_color_factor().into(),
            ..Default::default()
        };
        if let Some(texture) = pbr.base_color_texture() {
            material.base_color_texture = Some(data.load_base_color_image(&texture.texture()));
        }

        material.roughness_factor = pbr.roughness_factor();
        material.metallic_factor = pbr.metallic_factor();

        if let Some(texture) = pbr.metallic_roughness_texture() {
            if material.metallic_factor > 0. {
                material.metallic_texture = Some(data.load_gray_image(&texture.texture(), 2));
            }
            if material.roughness_factor > 0. {
                material.roughness_texture = Some(data.load_gray_image(&texture.texture(), 1));
            }
        }

        material
    }
}
impl Default for PbrMaterial {
    fn default() -> Self {
        PbrMaterial {
            base_color_factor: Vec4::ONE,
            base_color_texture: None,
            metallic_factor: 0.,
            metallic_texture: None,
            roughness_factor: 0.,
            roughness_texture: None,
        }
    }
}
