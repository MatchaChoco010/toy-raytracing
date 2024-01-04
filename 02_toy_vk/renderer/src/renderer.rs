use ash::vk;
use bytemuck;

use crate::NextImage;

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
struct Material {
    color: [f32; 3],
    ty: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    camera_rotate: glam::Mat4,
    camera_translate: glam::Vec3,
    seed: u32,
}

pub struct Renderer {
    width: u32,
    height: u32,
    max_sample_count: u32,
    rotate_x: f32,
    rotate_y: f32,
    rotate_z: f32,
    position_x: f32,
    position_y: f32,
    position_z: f32,

    instance: ashtray::InstanceHandle,
    physical_device: vk::PhysicalDevice,
    device: ashtray::DeviceHandle,
    queue_handles: ashtray::utils::QueueHandles,
    transfer_command_pool: ashtray::CommandPoolHandle,
    compute_command_pool: ashtray::CommandPoolHandle,
    transfer_command_buffer: ashtray::CommandBufferHandle,
    allocator: ashtray::AllocatorHandle,
    storage_image: ashtray::utils::ImageHandles,
    images: [ashtray::utils::ImageHandles; 2],
    sampler: ashtray::SamplerHandle,

    render_command_buffer: ashtray::CommandBufferHandle,
    in_flight_fence: ashtray::FenceHandle,

    blas_list: Option<Vec<ashtray::utils::BlasObjects>>,
    tlas: Option<ashtray::utils::TlasObjects>,
    ray_tracing_pipeline: Option<ashtray::RayTracingPipelineHandle>,
    ray_tracing_pipeline_layout: Option<ashtray::PipelineLayoutHandle>,
    ray_tracing_descriptor_set_layout: Option<ashtray::DescriptorSetLayoutHandle>,
    ray_tracing_descriptor_pool: Option<ashtray::DescriptorPoolHandle>,
    ray_tracing_descriptor_set: Option<ashtray::DescriptorSetHandle>,
    shader_binding_table: Option<ashtray::utils::ShaderBindingTable>,

    tonemap_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    tonemap_compute_pipeline: ashtray::ComputePipelineHandle,
    tonemap_command_buffers: [ashtray::CommandBufferHandle; 2],
    tonemap_descriptor_sets: [ashtray::DescriptorSetHandle; 2],
    tonemap_fences: [ashtray::FenceHandle; 2],

    current_image_index: usize,

    sample_count: u32,
}
impl Renderer {
    pub fn new(
        width: u32,
        height: u32,
        instance: ashtray::InstanceHandle,
        physical_device: vk::PhysicalDevice,
        device: ashtray::DeviceHandle,
        queue_handles: ashtray::utils::QueueHandles,
        graphics_command_pool: ashtray::CommandPoolHandle,
        allocator: ashtray::AllocatorHandle,
    ) -> Self {
        let transfer_command_pool =
            ashtray::utils::create_transfer_command_pool(&device, &queue_handles);
        let compute_command_pool =
            ashtray::utils::create_compute_command_pool(&device, &queue_handles);
        let transfer_command_buffer =
            ashtray::utils::allocate_command_buffers(&device, &transfer_command_pool, 1)
                .into_iter()
                .next()
                .unwrap();
        let storage_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let images = [
            ashtray::utils::create_shader_readonly_image(
                &device,
                &queue_handles,
                &allocator,
                &transfer_command_buffer,
                width,
                height,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            ),
            ashtray::utils::create_shader_readonly_image(
                &device,
                &queue_handles,
                &allocator,
                &transfer_command_buffer,
                width,
                height,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            ),
        ];
        let sampler = ashtray::utils::create_sampler(&device);

        // render用command bufferを作成
        let render_command_buffer = {
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(*graphics_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffers = device
                .allocate_command_buffers(&graphics_command_pool, &command_buffer_allocate_info);
            command_buffers.into_iter().next().unwrap()
        };

        // render用fenceの作成
        let in_flight_fence = ashtray::utils::create_signaled_fence(&device);

        let tonemap_descriptor_set_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .build(),
            ]),
        );
        let tonemap_compute_pipeline_layout = device.create_pipeline_layout(
            &tonemap_descriptor_set_layout,
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[*tonemap_descriptor_set_layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<u32>() as u32,
                }]),
        );
        let tonemap_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/tonemap.comp.spv")[..],
        );
        let tonemap_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &tonemap_compute_pipeline_layout,
            &tonemap_compute_shader_module,
        );
        let tonemap_command_pool =
            ashtray::utils::create_compute_command_pool(&device, &queue_handles);
        let tonemap_command_buffers: [ashtray::CommandBufferHandle; 2] =
            ashtray::utils::allocate_command_buffers(&device, &tonemap_command_pool, 2)
                .try_into()
                .unwrap();
        let tonemap_descriptor_pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .max_sets(2)
                .pool_sizes(&[
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::STORAGE_IMAGE,
                        descriptor_count: 1,
                    },
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::STORAGE_IMAGE,
                        descriptor_count: 1,
                    },
                ]),
        );
        let tonemap_descriptor_sets: [ashtray::DescriptorSetHandle; 2] = device
            .allocate_descriptor_sets(
                &tonemap_descriptor_pool,
                &tonemap_descriptor_set_layout,
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(*tonemap_descriptor_pool)
                    .set_layouts(&[*tonemap_descriptor_set_layout; 2]),
            )
            .try_into()
            .unwrap();
        for descriptor_set in tonemap_descriptor_sets.iter() {
            device.update_descriptor_sets(&[
                vk::WriteDescriptorSet::builder()
                    .dst_set(**descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(&[vk::DescriptorImageInfo {
                        sampler: vk::Sampler::null(),
                        image_view: *storage_image.image_view,
                        image_layout: vk::ImageLayout::GENERAL,
                    }])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(**descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(&[vk::DescriptorImageInfo {
                        sampler: vk::Sampler::null(),
                        image_view: *images[0].image_view,
                        image_layout: vk::ImageLayout::GENERAL,
                    }])
                    .build(),
            ]);
        }
        let tonemap_fences = [
            ashtray::utils::create_signaled_fence(&device),
            ashtray::utils::create_signaled_fence(&device),
        ];

        Self {
            width,
            height,
            max_sample_count: 256,
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,

            instance,
            physical_device,
            device,
            queue_handles,
            transfer_command_pool,
            compute_command_pool,
            transfer_command_buffer,
            allocator,
            storage_image,
            images,
            sampler,

            blas_list: None,
            tlas: None,
            ray_tracing_pipeline: None,
            ray_tracing_pipeline_layout: None,
            ray_tracing_descriptor_set_layout: None,
            ray_tracing_descriptor_pool: None,
            ray_tracing_descriptor_set: None,
            shader_binding_table: None,

            render_command_buffer,
            in_flight_fence,

            tonemap_compute_pipeline_layout,
            tonemap_compute_pipeline,
            tonemap_command_buffers,
            tonemap_descriptor_sets,
            tonemap_fences,

            current_image_index: 0,

            sample_count: 0,
        }
    }

    pub fn load_scene(&mut self, scene: &crate::Scene) {
        // blas/tlasの構築
        let blas_list = scene
            .meshes
            .iter()
            .map(|mesh| {
                let (models, _) = tobj::load_obj(&mesh.path, &tobj::GPU_LOAD_OPTIONS).unwrap();
                let mut vertices = vec![];
                let model = &models[0];
                let mesh = &model.mesh;
                for i in 0..mesh.positions.len() / 3 {
                    vertices.push(Vertex {
                        position: [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                        ],
                        normal: [
                            mesh.normals[i * 3],
                            mesh.normals[i * 3 + 1],
                            mesh.normals[i * 3 + 2],
                        ],
                    });
                }
                let indices = mesh.indices.clone();

                let blas = ashtray::utils::cerate_blas(
                    &self.device,
                    &self.queue_handles,
                    &self.compute_command_pool,
                    &self.allocator,
                    &vertices,
                    &indices,
                );

                blas
            })
            .collect::<Vec<_>>();

        let materials = scene
            .materials
            .iter()
            .map(|material| Material {
                color: material.color.into(),
                ty: material.ty as u32,
            })
            .collect::<Vec<_>>();

        let instances = scene
            .instances
            .iter()
            .map(|instance| {
                let transform = instance.transform;
                let blas = blas_list[instance.mesh_index].clone();
                (blas, transform, instance.material_index as u32)
            })
            .collect::<Vec<_>>();

        let tlas = ashtray::utils::create_tlas(
            &self.device,
            &self.queue_handles,
            &self.compute_command_pool,
            &self.transfer_command_pool,
            &self.allocator,
            &instances,
            &materials,
        );

        // let vertices = [
        //     Vertex {
        //         position: [0.0, -0.5, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //     },
        //     Vertex {
        //         position: [-0.5, 0.5, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //     },
        //     Vertex {
        //         position: [0.5, 0.5, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //     },
        // ];
        // let indices: [u32; 3] = [0, 1, 2];
        // let blas = ashtray::utils::cerate_blas(
        //     &self.device,
        //     &self.queue_handles,
        //     &self.compute_command_pool,
        //     &self.allocator,
        //     &vertices,
        //     &indices,
        // );

        // let instances = [(blas.clone(), glam::Mat4::IDENTITY, 0)];
        // let materials = [Material {
        //     color: [1.0, 0.0, 0.0],
        //     padding: 0,
        // }];
        // let tlas = ashtray::utils::create_tlas(
        //     &self.device,
        //     &self.queue_handles,
        //     &self.compute_command_pool,
        //     &self.transfer_command_pool,
        //     &self.allocator,
        //     &instances,
        //     &materials,
        // );

        // ray tracing pipelineの作成
        let raygen_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/raygen.rgen.spv"),
        );
        let miss_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/miss.rmiss.spv"),
        );
        let closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/closesthit.rchit.spv"),
        );
        let (ray_tracing_pipeline, pipeline_layout, descriptor_set_layout) =
            ashtray::utils::create_ray_tracing_pipelines(
                &self.device,
                &[raygen_shader_module],
                &[miss_shader_module],
                &[ashtray::utils::HitShaderModules {
                    closest_hit: closest_hit_shader_module,
                    any_hit: None,
                    intersection: None,
                }],
                &[vk::PushConstantRange::builder()
                    .stage_flags(
                        vk::ShaderStageFlags::RAYGEN_KHR
                            | vk::ShaderStageFlags::ANY_HIT_KHR
                            | vk::ShaderStageFlags::CLOSEST_HIT_KHR
                            | vk::ShaderStageFlags::MISS_KHR,
                    )
                    .offset(0)
                    .size(std::mem::size_of::<PushConstants>() as u32)
                    .build()],
            );

        // shader binding tableの作成
        let shader_binding_table = ashtray::utils::create_shader_binding_table(
            &self.instance,
            self.physical_device,
            &self.device,
            &self.allocator,
            &ray_tracing_pipeline,
            1,
            1,
            1,
        );

        // descriptor poolの作成
        let descriptor_pool = {
            let descriptor_pool_size_acceleration_structure = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1);
            let descriptor_pool_size_storage_image = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1);
            let descriptor_pool_size_instance_params = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1);
            let descriptor_pool_size_materials = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1);
            let descriptor_pool_sizes = [
                descriptor_pool_size_acceleration_structure.build(),
                descriptor_pool_size_storage_image.build(),
                descriptor_pool_size_instance_params.build(),
                descriptor_pool_size_materials.build(),
            ];

            let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&descriptor_pool_sizes)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

            self.device
                .create_descriptor_pool(&descriptor_pool_create_info)
        };

        // descriptor setの作成
        let descriptor_set = {
            // descriptor setのアロケート
            let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*descriptor_pool)
                .set_layouts(std::slice::from_ref(&descriptor_set_layout));
            let descriptor_set = self
                .device
                .allocate_descriptor_sets(
                    &descriptor_pool,
                    &descriptor_set_layout,
                    &descriptor_set_allocate_info,
                )
                .into_iter()
                .next()
                .unwrap();

            // acceleration structureの書き込み
            let mut descriptor_acceleration_structure_info =
                vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                    .acceleration_structures(std::slice::from_ref(&tlas.tlas));
            let mut acceleration_structure_write = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .push_next(&mut descriptor_acceleration_structure_info);
            acceleration_structure_write.descriptor_count = 1;

            // storage imageの書き込み
            let storage_image_info = vk::DescriptorImageInfo::builder()
                .image_view(*self.storage_image.image_view)
                .image_layout(vk::ImageLayout::GENERAL);
            let storage_image_write = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&storage_image_info));

            // instance paramsの書き込み
            let instance_params_info = vk::DescriptorBufferInfo::builder()
                .buffer(*tlas.instance_params_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);
            let instance_params_write = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(std::slice::from_ref(&instance_params_info));

            // materialsの書き込み
            let materials_info = vk::DescriptorBufferInfo::builder()
                .buffer(*tlas.materials_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);
            let materials_write = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(std::slice::from_ref(&materials_info));

            // descriptor setの更新
            let descriptor_writes = [
                acceleration_structure_write.build(),
                storage_image_write.build(),
                instance_params_write.build(),
                materials_write.build(),
            ];
            self.device.update_descriptor_sets(&descriptor_writes);

            descriptor_set
        };

        self.blas_list = Some(blas_list);
        self.tlas = Some(tlas);
        self.ray_tracing_pipeline = Some(ray_tracing_pipeline);
        self.ray_tracing_pipeline_layout = Some(pipeline_layout);
        self.ray_tracing_descriptor_set_layout = Some(descriptor_set_layout);
        self.ray_tracing_descriptor_pool = Some(descriptor_pool);
        self.ray_tracing_descriptor_set = Some(descriptor_set);
        self.shader_binding_table = Some(shader_binding_table);
    }

    fn set_parameters(&mut self, parameters: crate::Parameters) {
        if self.width != parameters.width || self.height != parameters.height {
            // width/heightが変わっていたらstorage imageをリサイズして作り直す。
            self.width = parameters.width;
            self.height = parameters.height;
            self.max_sample_count = parameters.max_sample_count;
            self.sample_count = 0;
            self.rotate_x = parameters.rotate_x;
            self.rotate_y = parameters.rotate_y;
            self.rotate_z = parameters.rotate_z;
            self.position_x = parameters.position_x;
            self.position_y = parameters.position_y;
            self.position_z = parameters.position_z;

            self.device.wait_idle();

            self.storage_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.width,
                self.height,
            );

            let command_buffer = self.render_command_buffer.clone();
            command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
            ashtray::utils::begin_onetime_command_buffer(&command_buffer);

            command_buffer.cmd_clear_color_image(
                &self.storage_image.image,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );

            command_buffer.end_command_buffer();
            let buffers_to_submit = [*command_buffer];
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&buffers_to_submit)
                .build();
            let fence = ashtray::utils::create_fence(&self.device);
            self.device.queue_submit(
                self.queue_handles.graphics.queue,
                &[submit_info],
                Some(fence.clone()),
            );
            self.device.wait_fences(&[fence], u64::MAX);

            if self.ray_tracing_pipeline.is_some() {
                let descriptor_set = self.ray_tracing_descriptor_set.as_ref().unwrap().clone();
                self.device
                    .update_descriptor_sets(&[vk::WriteDescriptorSet::builder()
                        .dst_set(*descriptor_set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                        .image_info(&[vk::DescriptorImageInfo {
                            sampler: vk::Sampler::null(),
                            image_view: *self.storage_image.image_view,
                            image_layout: vk::ImageLayout::GENERAL,
                        }])
                        .build()]);
            }

            self.images = [
                ashtray::utils::create_shader_readonly_image(
                    &self.device,
                    &self.queue_handles,
                    &self.allocator,
                    &self.transfer_command_buffer,
                    self.width,
                    self.height,
                    vk::Format::R8G8B8A8_UNORM,
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ),
                ashtray::utils::create_shader_readonly_image(
                    &self.device,
                    &self.queue_handles,
                    &self.allocator,
                    &self.transfer_command_buffer,
                    self.width,
                    self.height,
                    vk::Format::R8G8B8A8_UNORM,
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ),
            ];
            for (i, descriptor_set) in self.tonemap_descriptor_sets.iter().enumerate() {
                self.device.update_descriptor_sets(&[
                    vk::WriteDescriptorSet::builder()
                        .dst_set(**descriptor_set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                        .image_info(&[vk::DescriptorImageInfo {
                            sampler: vk::Sampler::null(),
                            image_view: *self.storage_image.image_view,
                            image_layout: vk::ImageLayout::GENERAL,
                        }])
                        .build(),
                    vk::WriteDescriptorSet::builder()
                        .dst_set(**descriptor_set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                        .image_info(&[vk::DescriptorImageInfo {
                            sampler: vk::Sampler::null(),
                            image_view: *self.images[i].image_view,
                            image_layout: vk::ImageLayout::GENERAL,
                        }])
                        .build(),
                ]);
            }
        } else if self.max_sample_count != parameters.max_sample_count
            || self.rotate_x != parameters.rotate_x
            || self.rotate_y != parameters.rotate_y
            || self.rotate_z != parameters.rotate_z
            || self.position_x != parameters.position_x
            || self.position_y != parameters.position_y
            || self.position_z != parameters.position_z
        {
            // そうでなくてdirtyなら蓄積をリセットするコマンドを発行する。
            self.max_sample_count = parameters.max_sample_count;
            self.sample_count = 0;
            self.rotate_x = parameters.rotate_x;
            self.rotate_y = parameters.rotate_y;
            self.rotate_z = parameters.rotate_z;
            self.position_x = parameters.position_x;
            self.position_y = parameters.position_y;
            self.position_z = parameters.position_z;

            let command_buffer = self.render_command_buffer.clone();
            command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
            ashtray::utils::begin_onetime_command_buffer(&command_buffer);

            command_buffer.cmd_clear_color_image(
                &self.storage_image.image,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );

            command_buffer.end_command_buffer();
            let buffers_to_submit = [*command_buffer];
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&buffers_to_submit)
                .build();
            let fence = ashtray::utils::create_fence(&self.device);
            self.device.queue_submit(
                self.queue_handles.graphics.queue,
                &[submit_info],
                Some(fence.clone()),
            );
            self.device.wait_fences(&[fence], u64::MAX);
        }
    }

    fn ray_trace(&mut self) {
        if self.sample_count >= self.max_sample_count {
            return;
        }

        if self.ray_tracing_pipeline.is_none() {
            return;
        }

        let shader_binding_table = self.shader_binding_table.as_ref().unwrap();
        let ray_tracing_pipeline = self.ray_tracing_pipeline.as_ref().unwrap();
        let ray_tracing_pipeline_layout = self.ray_tracing_pipeline_layout.as_ref().unwrap();
        let descriptor_set = self.ray_tracing_descriptor_set.as_ref().unwrap().clone();

        // command bufferの開始
        let command_buffer = self.render_command_buffer.clone();
        command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
        ashtray::utils::begin_onetime_command_buffer(&command_buffer);

        // sbt entryの用意
        let raygen_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(shader_binding_table.raygen_item.device_address)
            .stride(shader_binding_table.raygen_item.stride)
            .size(shader_binding_table.raygen_item.size);
        let miss_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(shader_binding_table.miss_item.device_address)
            .stride(shader_binding_table.miss_item.stride)
            .size(shader_binding_table.miss_item.size);
        let hit_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(shader_binding_table.hit_item.device_address)
            .stride(shader_binding_table.hit_item.stride)
            .size(shader_binding_table.hit_item.size);

        // ray tracing pipelineのbind
        command_buffer.cmd_bind_ray_tracing_pipeline(&ray_tracing_pipeline);

        // descriptor setのbind
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            ray_tracing_pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );

        command_buffer.cmd_push_constants(
            ray_tracing_pipeline_layout,
            vk::ShaderStageFlags::RAYGEN_KHR
                | vk::ShaderStageFlags::ANY_HIT_KHR
                | vk::ShaderStageFlags::CLOSEST_HIT_KHR
                | vk::ShaderStageFlags::MISS_KHR,
            0,
            &[PushConstants {
                camera_rotate: glam::Mat4::from_euler(
                    glam::EulerRot::YXZ,
                    self.rotate_y.to_radians(),
                    self.rotate_x.to_radians(),
                    self.rotate_z.to_radians(),
                ),
                camera_translate: glam::Vec3::new(
                    self.position_x,
                    self.position_y,
                    self.position_z,
                ),
                seed: self.sample_count as u32,
            }],
        );

        // ray tracingの実行
        command_buffer.cmd_trace_rays(
            &raygen_shader_sbt_entry,
            &miss_shader_sbt_entry,
            &hit_shader_sbt_entry,
            &vk::StridedDeviceAddressRegionKHR::default(),
            self.width,
            self.height,
            1,
        );

        command_buffer.end_command_buffer();
        let buffers_to_submit = [*command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&buffers_to_submit)
            .build();
        self.device.reset_fences(&[self.in_flight_fence.clone()]);
        self.device.queue_submit(
            self.queue_handles.graphics.queue,
            &[submit_info],
            Some(self.in_flight_fence.clone()),
        );
        self.device
            .wait_fences(&[self.in_flight_fence.clone()], u64::MAX);

        self.sample_count += 1;
    }

    // tonemapしつつtextureに結果を焼き込む
    fn take_image(&mut self) -> crate::NextImage {
        let image_handles = &self.images[self.current_image_index];
        let fences = [self.tonemap_fences[self.current_image_index].clone()];
        let command_buffer = self.tonemap_command_buffers[self.current_image_index].clone();

        self.device.wait_fences(&fences, u64::MAX);
        self.device.reset_fences(&fences);

        command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);

        ashtray::utils::begin_onetime_command_buffer(&command_buffer);

        ashtray::utils::cmd_image_barriers(
            &command_buffer,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags2::COMPUTE_SHADER,
            vk::AccessFlags2::SHADER_STORAGE_WRITE,
            vk::ImageLayout::GENERAL,
            &image_handles.image,
        );

        command_buffer.cmd_bind_compute_pipeline(&self.tonemap_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.tonemap_compute_pipeline_layout,
            0,
            &[self.tonemap_descriptor_sets[self.current_image_index].clone()],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.tonemap_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[self.sample_count],
        );
        command_buffer.cmd_dispatch((self.width + 7) / 8, (self.height + 7) / 8, 1);

        ashtray::utils::cmd_image_barriers(
            &command_buffer,
            vk::PipelineStageFlags2::COMPUTE_SHADER,
            vk::AccessFlags2::SHADER_SAMPLED_READ,
            vk::ImageLayout::GENERAL,
            vk::PipelineStageFlags2::COMPUTE_SHADER,
            vk::AccessFlags2::SHADER_WRITE,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            &image_handles.image,
        );

        command_buffer.end_command_buffer();

        self.device.queue_submit(
            self.queue_handles.compute.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[*command_buffer])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])
                    .wait_semaphores(&[]),
            ),
            Some(self.tonemap_fences[self.current_image_index].clone()),
        );

        let image_view = image_handles.image_view.clone();
        let sampler = self.sampler.clone();
        let sample_count = self.sample_count;

        self.current_image_index = (self.current_image_index + 1) % 2;

        NextImage {
            image_view,
            sampler,
            sample_count,
        }
    }

    pub fn render(&mut self, parameters: crate::Parameters) -> NextImage {
        self.set_parameters(parameters);
        self.ray_trace();
        self.take_image()
    }
}
