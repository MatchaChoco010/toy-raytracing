use ash::vk;
use image::Pixel;

pub struct Glb {
    pub path: String,
}

pub struct Instance {
    pub transform: glam::Mat4,
    pub glb_index: usize,
}

pub struct Scene {
    pub glbs: Vec<Glb>,
    pub instances: Vec<Instance>,
}

#[repr(C)]
pub(crate) struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 4],
    tex_coords: [f32; 2],
}

#[repr(C)]
pub(crate) struct Material {
    base_color_factor: [f32; 4],
    base_color_texture_index: i32,
    emissive_factor: [f32; 3],
    emissive_texture_index: i32,
    metallic_factor: f32,
    metallic_texture_index: i32,
    roughness_factor: f32,
    roughness_texture_index: i32,
    normal_factor: f32,
    normal_texture_index: i32,
    ty: u32,
}

pub(crate) struct SceneObjects {
    pub(crate) _sampler: ashtray::SamplerHandle,
    pub(crate) _images: Vec<ashtray::utils::ImageHandles>,
    pub(crate) _blas_list: Vec<ashtray::utils::BlasObjects>,
    pub(crate) tlas: ashtray::utils::TlasObjects,
}
pub(crate) fn load_scene(
    device: &ashtray::DeviceHandle,
    queue_handles: &ashtray::utils::QueueHandles,
    compute_command_pool: &ashtray::CommandPoolHandle,
    transfer_command_pool: &ashtray::CommandPoolHandle,
    allocator: &ashtray::AllocatorHandle,
    descriptor_sets: &ashtray::utils::BindlessDescriptorSets,
    scene: &Scene,
) -> SceneObjects {
    let sampler = ashtray::utils::create_sampler(device);
    let mut images = vec![];
    let mut blas_list = vec![];
    let mut materials = vec![];

    for glb in &scene.glbs {
        let glb_scenes = easy_gltf::load(&glb.path).expect("Failed to load glb file");

        for glb_scene in glb_scenes {
            for model in &glb_scene.models {
                let vertices = model.vertices();
                let indices = model.indices().unwrap();
                let material = model.material();

                let vertices = vertices
                    .iter()
                    .map(|v| Vertex {
                        position: [v.position.x, v.position.y, v.position.z],
                        normal: [v.normal.x, v.normal.y, v.normal.z],
                        tangent: [v.tangent.x, v.tangent.y, v.tangent.z, v.tangent.w],
                        tex_coords: [v.tex_coords.x, v.tex_coords.y],
                    })
                    .collect::<Vec<_>>();

                let blas = ashtray::utils::cerate_blas(
                    device,
                    queue_handles,
                    compute_command_pool,
                    allocator,
                    &vertices,
                    &indices,
                );
                blas_list.push(blas);

                let base_color_factor = material.pbr.base_color_factor;
                let base_color_texture_index =
                    if let Some(texture) = &material.pbr.base_color_texture {
                        let data = texture
                            .enumerate_pixels()
                            .flat_map(|(_x, _y, p)| p.to_rgba().0)
                            .collect::<Vec<_>>();
                        let image = ashtray::utils::create_shader_readonly_image_with_data(
                            device,
                            queue_handles,
                            allocator,
                            transfer_command_pool,
                            texture.width(),
                            texture.height(),
                            &data,
                            vk::Format::R8G8B8A8_SRGB,
                            vk::ImageUsageFlags::SAMPLED,
                        );
                        let image_index = images.len();

                        descriptor_sets.combined_image_sampler.update(
                            &image,
                            &sampler,
                            image_index as u32,
                        );

                        images.push(image);
                        image_index as i32
                    } else {
                        -1
                    };

                let metallic_factor = material.pbr.metallic_factor;
                let metallic_texture_index = if let Some(texture) = &material.pbr.metallic_texture {
                    let data = texture
                        .enumerate_pixels()
                        .flat_map(|(_x, _y, p)| p.to_rgba().0)
                        .collect::<Vec<_>>();
                    let image = ashtray::utils::create_shader_readonly_image_with_data(
                        device,
                        queue_handles,
                        allocator,
                        transfer_command_pool,
                        texture.width(),
                        texture.height(),
                        &data,
                        vk::Format::R8G8B8A8_UNORM,
                        vk::ImageUsageFlags::SAMPLED,
                    );
                    let image_index = images.len();

                    descriptor_sets.combined_image_sampler.update(
                        &image,
                        &sampler,
                        image_index as u32,
                    );

                    images.push(image);
                    image_index as i32
                } else {
                    -1
                };

                let roughness_factor = material.pbr.roughness_factor;
                let roughness_texture_index = if let Some(texture) = &material.pbr.roughness_texture
                {
                    let data = texture
                        .enumerate_pixels()
                        .flat_map(|(_x, _y, p)| p.to_rgba().0)
                        .collect::<Vec<_>>();
                    let image = ashtray::utils::create_shader_readonly_image_with_data(
                        device,
                        queue_handles,
                        allocator,
                        transfer_command_pool,
                        texture.width(),
                        texture.height(),
                        &data,
                        vk::Format::R8G8B8A8_UNORM,
                        vk::ImageUsageFlags::SAMPLED,
                    );
                    let image_index = images.len();

                    descriptor_sets.combined_image_sampler.update(
                        &image,
                        &sampler,
                        image_index as u32,
                    );

                    images.push(image);
                    image_index as i32
                } else {
                    -1
                };

                let normal_factor = if let Some(normal) = &material.normal {
                    normal.factor
                } else {
                    1.0
                };
                let normal_texture_index = if let Some(normal) = &material.normal {
                    let texture = &normal.texture;
                    let data = texture
                        .enumerate_pixels()
                        .flat_map(|(_x, _y, p)| p.to_rgba().0)
                        .collect::<Vec<_>>();
                    let image = ashtray::utils::create_shader_readonly_image_with_data(
                        device,
                        queue_handles,
                        allocator,
                        transfer_command_pool,
                        texture.width(),
                        texture.height(),
                        &data,
                        vk::Format::R8G8B8A8_SRGB,
                        vk::ImageUsageFlags::SAMPLED,
                    );
                    let image_index = images.len();

                    descriptor_sets.combined_image_sampler.update(
                        &image,
                        &sampler,
                        image_index as u32,
                    );

                    images.push(image);
                    image_index as i32
                } else {
                    -1
                };

                let emissive_factor = material.emissive.factor * 10.0;
                let emissive_texture_index = if let Some(texture) = &material.emissive.texture {
                    let data = texture
                        .enumerate_pixels()
                        .flat_map(|(_x, _y, p)| p.to_rgba().0)
                        .collect::<Vec<_>>();
                    let image = ashtray::utils::create_shader_readonly_image_with_data(
                        device,
                        queue_handles,
                        allocator,
                        transfer_command_pool,
                        texture.width(),
                        texture.height(),
                        &data,
                        vk::Format::R8G8B8A8_UNORM,
                        vk::ImageUsageFlags::SAMPLED,
                    );
                    let image_index = images.len();

                    descriptor_sets.combined_image_sampler.update(
                        &image,
                        &sampler,
                        image_index as u32,
                    );

                    images.push(image);
                    image_index as i32
                } else {
                    -1
                };

                let material = Material {
                    base_color_factor: [
                        base_color_factor.x,
                        base_color_factor.y,
                        base_color_factor.z,
                        base_color_factor.w,
                    ],
                    base_color_texture_index,
                    metallic_factor,
                    metallic_texture_index,
                    roughness_factor,
                    roughness_texture_index,
                    normal_factor,
                    normal_texture_index,
                    emissive_factor: [emissive_factor.x, emissive_factor.y, emissive_factor.z],
                    emissive_texture_index,
                    ty: 0,
                };

                materials.push(material);
            }
        }
    }

    let mut instances = vec![];
    for instance in &scene.instances {
        let transform = instance.transform;
        let glb_index = instance.glb_index;
        let blas = blas_list[glb_index].clone();

        instances.push((blas, transform, glb_index as u32));
    }

    let tlas = ashtray::utils::create_tlas(
        device,
        queue_handles,
        compute_command_pool,
        transfer_command_pool,
        allocator,
        &instances,
        &materials,
    );

    SceneObjects {
        _sampler: sampler,
        _images: images,
        _blas_list: blas_list,
        tlas,
    }
}
