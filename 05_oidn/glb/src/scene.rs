pub mod model;

use glam::Mat4;
use gltf::scene::Node;

use crate::GlbData;

pub use model::{AlphaMode, Model};

#[derive(Default, Clone, Debug)]
pub struct Scene {
    pub models: Vec<Model>,
}

impl Scene {
    pub(crate) fn load(gltf_scene: gltf::Scene, data: &mut GlbData) -> Self {
        let mut scene = Self::default();

        for node in gltf_scene.nodes() {
            scene.read_node(&node, Mat4::IDENTITY, data);
        }
        scene
    }

    fn read_node(&mut self, node: &Node, parent_transform: Mat4, data: &mut GlbData) {
        let transform = parent_transform * Mat4::from_cols_array_2d(&node.transform().matrix());

        for child in node.children() {
            self.read_node(&child, transform, data);
        }

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                self.models.push(Model::load(primitive, transform, data));
            }
        }
    }
}
