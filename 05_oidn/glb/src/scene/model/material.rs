use glam::{Vec3, Vec4};
use image::{GrayImage, RgbImage, RgbaImage};
use std::sync::Arc;

use crate::GlbData;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub pbr: PbrMaterial,
    pub normal: Option<NormalMap>,
    pub emissive: Emissive,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
}
impl Material {
    pub(crate) fn load(gltf_mat: gltf::Material, data: &mut GlbData) -> Arc<Self> {
        if let Some(material) = data.materials.get(&gltf_mat.index()) {
            return material.clone();
        }

        let alpha_mode = match gltf_mat.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        };
        let alpha_cutoff = gltf_mat.alpha_cutoff().unwrap_or(0.5);

        let material = Arc::new(Material {
            pbr: PbrMaterial::load(gltf_mat.pbr_metallic_roughness(), data),
            normal: NormalMap::load(&gltf_mat, data),
            emissive: Emissive::load(&gltf_mat, data),
            alpha_mode,
            alpha_cutoff,
        });

        // Add to the collection
        data.materials.insert(gltf_mat.index(), material.clone());
        material
    }
}
