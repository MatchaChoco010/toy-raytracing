mod glb_data;

pub(crate) use glb_data::GlbData;

use glam::Mat4;
use gltf::scene::Transform;

pub fn transform_to_matrix(transform: Transform) -> Mat4 {
    let tr = transform.matrix();
    Mat4::from_cols_array_2d(&tr)
}
