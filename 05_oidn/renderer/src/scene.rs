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
    pub glb_list: Vec<Glb>,
    pub instances: Vec<Instance>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 3],
    tex_coords: [f32; 2],
}

#[derive(Debug, Clone, Copy)]
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
    alpha_cutoff: f32,
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
    pub(crate) sky_texture_cdf_row_buffer: ashtray::utils::BufferObjects,
    pub(crate) sky_texture_pdf_row_buffer: ashtray::utils::BufferObjects,
    pub(crate) sky_texture_cdf_column_buffer: ashtray::utils::BufferObjects,
    pub(crate) sky_texture_pdf_column_buffer: ashtray::utils::BufferObjects,
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
    let mut blas_lists = vec![];
    let mut materials = vec![];
    let mut materials_offset_indices = vec![];
    let mut instances = vec![];

    for glb in &scene.glb_list {
        let glb_scenes = glb::load(&glb.path).expect("Failed to load glb file");

        let mut glb_blas_list = vec![];
        materials_offset_indices.push(materials.len());

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

                let ty = match material.alpha_mode {
                    glb::AlphaMode::Opaque => 0,
                    glb::AlphaMode::Mask => 1,
                    glb::AlphaMode::Blend => 2,
                };
                let transparent_flag = material.alpha_mode != glb::AlphaMode::Opaque;

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
                    alpha_cutoff: material.alpha_cutoff,
                    ty,
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
                glb_blas_list.push(blas);
            }
        }
        blas_lists.push(glb_blas_list);
    }

    for instance in &scene.instances {
        let transform = instance.transform;
        let glb_index = instance.glb_index;
        let blas_list = blas_lists[glb_index].clone();
        let materials_offset_index = materials_offset_indices[glb_index];

        for i in 0..blas_list.len() {
            let blas = blas_list[i].clone();
            let material_index = materials_offset_index + i;
            let material = materials[material_index].clone();
            let sbt_offset = material.ty as u32;

            instances.push((blas, transform, material_index as u32, sbt_offset));
        }
    }

    let blas_list = blas_lists
        .iter()
        .flatten()
        .map(|b| b.clone())
        .collect::<Vec<_>>();

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

    fn luminance(rgb: glam::Vec3) -> f64 {
        0.2126 * rgb.x as f64 + 0.7152 * rgb.y as f64 + 0.0722 * rgb.z as f64
    }

    let mut sky_cdf_row_sum_data =
        vec![vec![0.0f64; sky_texture_width as usize + 1]; sky_texture_height as usize];
    for y in 0..sky_texture_height as usize {
        for x in 0..sky_texture_width as usize {
            // 緯度経度のテクスチャ座標から一様サンプリングするために重点サンプリングにウェイトをかける
            let weight = (std::f64::consts::PI
                * ((y as f64 + 0.5) / sky_texture_height as f64) as f64)
                .sin()
                * 2.0
                * std::f64::consts::PI;
            let index = y * (sky_texture_width as usize) + x;
            sky_cdf_row_sum_data[y][x + 1] = sky_cdf_row_sum_data[y][x]
                + weight
                    * luminance(glam::vec3(
                        sky_data[index * 3],
                        sky_data[index * 3 + 1],
                        sky_data[index * 3 + 2],
                    ));
        }
    }
    luminance(glam::vec3(sky_data[0], sky_data[1], sky_data[2]));
    let mut sky_cdf_row_data =
        vec![vec![0.0f64; sky_texture_width as usize + 1]; sky_texture_height as usize];
    for y in 0..sky_texture_height as usize {
        for x in 0..sky_texture_width as usize + 1 {
            sky_cdf_row_data[y][x] =
                sky_cdf_row_sum_data[y][x] / sky_cdf_row_sum_data[y][sky_texture_width as usize];
        }
    }
    let sky_cdf_row_data_flatten = sky_cdf_row_data
        .iter()
        .flatten()
        .map(|v| *v as f32)
        .collect::<Vec<_>>();
    let sky_texture_cdf_row_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_cdf_row_data_flatten,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    let mut sky_pdf_row_data =
        vec![vec![0.0f64; sky_texture_width as usize]; sky_texture_height as usize];
    for y in 0..sky_texture_height as usize {
        for x in 0..sky_texture_width as usize {
            sky_pdf_row_data[y][x] = sky_cdf_row_data[y][x + 1] - sky_cdf_row_data[y][x];
        }
    }
    let sky_pdf_row_data_flatten_raw = sky_pdf_row_data
        .iter()
        .flatten()
        .map(|v| *v as f32)
        .collect::<Vec<_>>();
    let sky_texture_pdf_row_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_pdf_row_data_flatten_raw,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    let mut sky_cdf_column_sum_data = vec![0.0f64; sky_texture_height as usize + 1];
    for y in 0..sky_texture_height as usize {
        sky_cdf_column_sum_data[y + 1] =
            sky_cdf_column_sum_data[y] + sky_cdf_row_sum_data[y][sky_texture_width as usize];
    }
    let mut sky_cdf_column_data = vec![0.0f64; sky_texture_height as usize + 1];
    for y in 0..sky_texture_height as usize {
        sky_cdf_column_data[y + 1] =
            sky_cdf_column_sum_data[y + 1] / sky_cdf_column_sum_data[sky_texture_height as usize];
    }
    let sky_cdf_column_data_raw = sky_cdf_column_data
        .iter()
        .map(|v| *v as f32)
        .collect::<Vec<_>>();
    let sky_texture_cdf_column_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_cdf_column_data_raw,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    let mut sky_pdf_column_data = vec![0.0f64; sky_texture_height as usize];
    for y in 0..sky_texture_height as usize {
        sky_pdf_column_data[y] = sky_cdf_column_data[y + 1] - sky_cdf_column_data[y];
    }
    let sky_pdf_column_data = sky_pdf_column_data
        .iter()
        .map(|v| *v as f32)
        .collect::<Vec<_>>();
    let sky_texture_pdf_column_buffer = ashtray::utils::create_device_local_buffer_with_data(
        device,
        queue_handles,
        transfer_command_pool,
        allocator,
        &sky_pdf_column_data,
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
        sky_texture_cdf_row_buffer,
        sky_texture_pdf_row_buffer,
        sky_texture_cdf_column_buffer,
        sky_texture_pdf_column_buffer,
    }
}
