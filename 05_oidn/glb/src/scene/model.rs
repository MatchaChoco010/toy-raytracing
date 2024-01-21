mod material;
mod vertex;

use glam::{vec4, Mat4, Vec2, Vec3, Vec4};
use std::sync::Arc;

use crate::GlbData;
pub use material::*;
pub use vertex::*;

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Option<Vec<u32>>,
    pub(crate) material: Arc<Material>,
}

impl Model {
    pub fn material(&self) -> Arc<Material> {
        self.material.clone()
    }

    pub fn vertices(&self) -> &Vec<Vertex> {
        &self.vertices
    }

    pub fn indices(&self) -> Option<&Vec<u32>> {
        self.indices.as_ref()
    }

    fn apply_transform_tangent(tangent: [f32; 4], transform: Mat4) -> Vec4 {
        let tang = vec4(tangent[0], tangent[1], tangent[2], 0.0);
        let mut tang = transform * tang;
        tang[3] = tangent[3];
        tang
    }

    pub(crate) fn load(primitive: gltf::Primitive, transform: Mat4, data: &mut GlbData) -> Self {
        let buffers = &data.buffers;
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let indices = reader
            .read_indices()
            .map(|indices| indices.into_u32().collect());

        let mut vertices: Vec<_> = reader
            .read_positions()
            .unwrap_or_else(|| panic!("The model primitive doesn't contain positions"))
            .map(|pos| Vertex {
                position: transform.transform_point3(Vec3::from_array(pos)),
                ..Default::default()
            })
            .collect();

        if let Some(normals) = reader.read_normals() {
            for (i, normal) in normals.enumerate() {
                vertices[i].normal = transform
                    .transform_vector3(Vec3::from_array(normal))
                    .normalize();
            }
        }
        if let Some(tangents) = reader.read_tangents() {
            for (i, tangent) in tangents.enumerate() {
                let tangent = Self::apply_transform_tangent(tangent, transform);
                vertices[i].tangent = tangent.truncate().normalize().extend(tangent.w);
            }
        }

        if let Some(tex_coords) = reader.read_tex_coords(0) {
            for (i, tex_coords) in tex_coords.into_f32().enumerate() {
                vertices[i].tex_coords = Vec2::from(tex_coords);
            }
        }

        Model {
            vertices,
            indices,
            material: Material::load(primitive.material(), data),
        }
    }
}
