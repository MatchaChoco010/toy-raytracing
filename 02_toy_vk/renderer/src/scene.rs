pub struct Material {
    pub color: glam::Vec3,
    pub ty: u32,
}

pub struct Mesh {
    pub path: String,
}

pub struct Instance {
    pub transform: glam::Mat4,
    pub mesh_index: usize,
    pub material_index: usize,
}

pub struct Scene {
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
    pub instances: Vec<Instance>,
}
