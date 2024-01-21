mod emissive;
mod normal;
mod pbr;

use crate::glb::utils::*;
use std::sync::Arc;

pub use emissive::Emissive;
pub use normal::NormalMap;
pub use pbr::PbrMaterial;

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
