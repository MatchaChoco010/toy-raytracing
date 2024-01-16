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
    pub sky_texture_path: String,
    pub glbs: Vec<Glb>,
    pub instances: Vec<Instance>,
}

#[repr(C)]
pub(crate) struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 3],
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
    pub(crate) sky_texture_width: u32,
    pub(crate) sky_texture_height: u32,
    pub(crate) sky_texture_buffer: ashtray::utils::BufferObjects,
    pub(crate) sky_texture_cdf_buffer: ashtray::utils::BufferObjects,
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
    let sampler = ashtray::utils::create_sampler_image(device);
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

                let mut vertices = vertices
                    .iter()
                    .map(|v| Vertex {
                        position: [v.position.x, v.position.y, v.position.z],
                        normal: [v.normal.x, v.normal.y, v.normal.z],
                        tangent: [0.0, 0.0, 0.0],
                        tex_coords: [v.tex_coords.x, v.tex_coords.y],
                    })
                    .collect::<Vec<_>>();
                // UVからtangentの計算
                for index in indices.chunks(3) {
                    let idx0 = index[0] as usize;
                    let idx1 = index[1] as usize;
                    let idx2 = index[2] as usize;
                    let dv1 = glam::Vec3::from_array(vertices[idx1].position)
                        - glam::Vec3::from_array(vertices[idx0].position);
                    let dv2 = glam::Vec3::from_array(vertices[idx2].position)
                        - glam::Vec3::from_array(vertices[idx0].position);
                    let duv1 = glam::Vec2::from_array(vertices[idx1].tex_coords)
                        - glam::Vec2::from_array(vertices[idx0].tex_coords);
                    let duv2 = glam::Vec2::from_array(vertices[idx2].tex_coords)
                        - glam::Vec2::from_array(vertices[idx0].tex_coords);
                    let r = 1.0 / (duv1.x * duv2.y - duv1.y * duv2.x);
                    let tangent = (dv1 * duv2.y - dv2 * duv1.y) * r;

                    vertices[idx0].tangent = tangent.to_array();
                    vertices[idx1].tangent = tangent.to_array();
                    vertices[idx2].tangent = tangent.to_array();
                }

                let mut transparent_flag = false;
                let base_color_factor = material.pbr.base_color_factor;
                transparent_flag |= base_color_factor.w < 1.0;
                let base_color_texture_index =
                    if let Some(texture) = &material.pbr.base_color_texture {
                        let data = texture
                            .enumerate_pixels()
                            .flat_map(|(_x, _y, p)| {
                                transparent_flag |= p.to_rgba().0[3] < 255;
                                p.to_rgba().0
                            })
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

                let emissive_factor = material.emissive.factor * 1000.0;
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
                    ty: if transparent_flag { 1 } else { 0 },
                };
                materials.push(material);

                let blas = ashtray::utils::cerate_blas(
                    device,
                    queue_handles,
                    compute_command_pool,
                    allocator,
                    &vertices,
                    &indices,
                    transparent_flag,
                );
                blas_list.push(blas);
            }
        }
    }

    let mut instances = vec![];
    for instance in &scene.instances {
        let transform = instance.transform;
        let glb_index = instance.glb_index;
        let blas = blas_list[glb_index].clone();
        let sbt_offset = materials[glb_index].ty as u32;

        instances.push((blas, transform, glb_index as u32, sbt_offset));
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

    let sky_texture = image::open(&scene.sky_texture_path).unwrap();
    let sky_texture_width = sky_texture.width();
    let sky_texture_height = sky_texture.height();
    let sky_data = sky_texture
        .as_rgb32f()
        .expect("Failed to load sky texture, only RGB32F is supported")
        .enumerate_pixels()
        .flat_map(|(_x, _y, p)| p.0)
        .collect::<Vec<_>>();
    let sky_texture_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_data,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );
    let sky_cdf_data =
        vec![vec![1.0f32; sky_texture_width as usize + 1]; sky_texture_height as usize + 1];
    let sky_texture_cdf_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_cdf_data,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    SceneObjects {
        _sampler: sampler,
        _images: images,
        _blas_list: blas_list,
        tlas,
        sky_texture_width,
        sky_texture_height,
        sky_texture_buffer,
        sky_texture_cdf_buffer,
    }
}
