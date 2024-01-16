use ash::vk;
use bytemuck;

use crate::NextImage;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    camera_rotate: glam::Mat4,
    camera_translate: glam::Vec3,
    camera_fov: f32,
    sample_index: u32,
    max_recursion_depth: u32,
    storage_image_index: u32,
    instance_params_index: u32,
    materials_index: u32,
    sun_strength: f32,
    padding_0: [u32; 2],
    sun_color: glam::Vec3,
    padding_1: [u32; 1],
    sun_direction: glam::Vec2,
    sun_angle: f32,
    sun_enabled: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FinalPushConstants {
    sample_count: u32,
    input_index: u32,
    output_index: u32,
    l_white: f32,
    aperture: f32,
    shutter_speed: f32,
    iso: f32,
}

pub struct Renderer {
    params: crate::Parameters,

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

    descriptor_sets: ashtray::utils::BindlessDescriptorSets,
    accumulate_storage_image_index: u32,
    final_storage_image_indices: [u32; 2],

    scene_objects: Option<crate::scene::SceneObjects>,

    ray_tracing_pipeline: Option<ashtray::RayTracingPipelineHandle>,
    ray_tracing_pipeline_layout: Option<ashtray::PipelineLayoutHandle>,
    acceleration_structure_descriptor_set:
        Option<ashtray::utils::DescriptorSetAccelerationStructureHandles>,
    shader_binding_table: Option<ashtray::utils::ShaderBindingTable>,
    instance_params_buffer_index: Option<u32>,
    materials_buffer_index: Option<u32>,

    final_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    final_compute_pipeline: ashtray::ComputePipelineHandle,
    final_command_buffers: [ashtray::CommandBufferHandle; 2],
    final_fences: [ashtray::FenceHandle; 2],

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

        // bindlessなdescriptor setsを作成
        let descriptor_sets = ashtray::utils::BindlessDescriptorSets::create(&device);
        let accumulate_storage_image_index = 0;
        descriptor_sets
            .storage_image
            .update(&storage_image, accumulate_storage_image_index);
        let final_storage_image_indices = [1, 2];
        descriptor_sets
            .storage_image
            .update(&images[0], final_storage_image_indices[0]);
        descriptor_sets
            .storage_image
            .update(&images[1], final_storage_image_indices[1]);

        let final_compute_pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[*descriptor_sets.storage_image.layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<FinalPushConstants>() as u32,
                }]),
        );
        let final_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/final.comp.spv")[..],
        );
        let final_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &final_compute_pipeline_layout,
            &final_compute_shader_module,
        );
        let final_command_pool =
            ashtray::utils::create_compute_command_pool(&device, &queue_handles);
        let final_command_buffers: [ashtray::CommandBufferHandle; 2] =
            ashtray::utils::allocate_command_buffers(&device, &final_command_pool, 2)
                .try_into()
                .unwrap();
        let final_fences = [
            ashtray::utils::create_signaled_fence(&device),
            ashtray::utils::create_signaled_fence(&device),
        ];

        Self {
            params: crate::Parameters::default(),

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

            descriptor_sets,
            accumulate_storage_image_index,
            final_storage_image_indices,

            scene_objects: None,
            ray_tracing_pipeline: None,
            ray_tracing_pipeline_layout: None,
            acceleration_structure_descriptor_set: None,
            shader_binding_table: None,
            instance_params_buffer_index: None,
            materials_buffer_index: None,

            render_command_buffer,
            in_flight_fence,

            final_compute_pipeline_layout,
            final_compute_pipeline,
            final_command_buffers,
            final_fences,

            current_image_index: 0,

            sample_count: 0,
        }
    }

    pub fn load_scene(&mut self, scene: &crate::Scene) {
        let scene_objects = crate::scene::load_scene(
            &self.device,
            &self.queue_handles,
            &self.compute_command_pool,
            &self.transfer_command_pool,
            &self.allocator,
            &self.descriptor_sets,
            scene,
        );

        let instance_params_buffer_index = 0;
        self.descriptor_sets.storage_buffer.update(
            &scene_objects.tlas.instance_params_buffer.buffer,
            instance_params_buffer_index,
        );
        let materials_buffer_index = 1;
        self.descriptor_sets.storage_buffer.update(
            &scene_objects.tlas.materials_buffer.buffer,
            materials_buffer_index,
        );

        // acceleration structureのdescriptor setの作成
        let acceleration_structure_descriptor_set =
            ashtray::utils::DescriptorSetAccelerationStructureHandles::create(
                &self.device,
                &scene_objects.tlas.tlas,
            );

        // ray tracing pipelineの作成
        let raygen_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/raygen.rgen.spv"),
        );
        let material_miss_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/material/miss.rmiss.spv"),
        );
        let material_closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/material/closesthit.rchit.spv"),
        );
        let shadow_closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/closesthit.rchit.spv"),
        );
        let shadow_anyhit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/anyhit.rahit.spv"),
        );
        let shadow_miss_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/miss.rmiss.spv"),
        );
        let (ray_tracing_pipeline, pipeline_layout, shader_binding_table) =
            ashtray::utils::create_ray_tracing_pipelines(
                &self.instance,
                self.physical_device,
                &self.device,
                &self.allocator,
                &[raygen_shader_module],
                &[material_miss_shader_module, shadow_miss_shader_module],
                &[
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(material_closest_hit_shader_module.clone()),
                        any_hit: None,
                        intersection: None,
                    },
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(material_closest_hit_shader_module),
                        any_hit: None,
                        intersection: None,
                    },
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(shadow_closest_hit_shader_module.clone()),
                        any_hit: None,
                        intersection: None,
                    },
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(shadow_closest_hit_shader_module),
                        any_hit: Some(shadow_anyhit_shader_module),
                        intersection: None,
                    },
                ],
                &[
                    *self.descriptor_sets.uniform_buffer.layout.clone(),
                    *self.descriptor_sets.combined_image_sampler.layout.clone(),
                    *self.descriptor_sets.storage_buffer.layout.clone(),
                    *self.descriptor_sets.storage_image.layout.clone(),
                    *acceleration_structure_descriptor_set.layout.clone(),
                ],
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

        self.scene_objects = Some(scene_objects);
        self.ray_tracing_pipeline = Some(ray_tracing_pipeline);
        self.ray_tracing_pipeline_layout = Some(pipeline_layout);
        self.acceleration_structure_descriptor_set = Some(acceleration_structure_descriptor_set);
        self.shader_binding_table = Some(shader_binding_table);
        self.instance_params_buffer_index = Some(instance_params_buffer_index);
        self.materials_buffer_index = Some(materials_buffer_index);
    }

    fn set_parameters(&mut self, parameters: crate::Parameters) {
        if self.params.width != parameters.width || self.params.height != parameters.height {
            // width/heightが変わっていたらstorage imageをリサイズして作り直す。
            self.params = parameters;
            self.sample_count = 0;

            self.device.wait_idle();

            self.storage_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
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

            self.images = [
                ashtray::utils::create_shader_readonly_image(
                    &self.device,
                    &self.queue_handles,
                    &self.allocator,
                    &self.transfer_command_buffer,
                    self.params.width,
                    self.params.height,
                    vk::Format::R8G8B8A8_UNORM,
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ),
                ashtray::utils::create_shader_readonly_image(
                    &self.device,
                    &self.queue_handles,
                    &self.allocator,
                    &self.transfer_command_buffer,
                    self.params.width,
                    self.params.height,
                    vk::Format::R8G8B8A8_UNORM,
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ),
            ];

            self.descriptor_sets
                .storage_image
                .update(&self.storage_image, self.accumulate_storage_image_index);
            self.descriptor_sets
                .storage_image
                .update(&self.images[0], self.final_storage_image_indices[0]);
            self.descriptor_sets
                .storage_image
                .update(&self.images[1], self.final_storage_image_indices[1]);
        } else if self.params != parameters {
            // そうでなくてdirtyなら蓄積をリセットするコマンドを発行する。
            self.params = parameters;
            self.sample_count = 0;

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
        if self.sample_count >= self.params.max_sample_count {
            return;
        }

        if self.ray_tracing_pipeline.is_none() {
            return;
        }

        let shader_binding_table = self.shader_binding_table.as_ref().unwrap();
        let ray_tracing_pipeline = self.ray_tracing_pipeline.as_ref().unwrap();
        let ray_tracing_pipeline_layout = self.ray_tracing_pipeline_layout.as_ref().unwrap();
        let descriptor_sets = self.acceleration_structure_descriptor_set.as_ref().unwrap();
        let instance_params_index = self.instance_params_buffer_index.unwrap();
        let materials_index = self.materials_buffer_index.unwrap();

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
            &[
                self.descriptor_sets.uniform_buffer.set.clone(),
                self.descriptor_sets.combined_image_sampler.set.clone(),
                self.descriptor_sets.storage_buffer.set.clone(),
                self.descriptor_sets.storage_image.set.clone(),
                descriptor_sets.set.clone(),
            ],
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
                    self.params.rotate_y.to_radians(),
                    self.params.rotate_x.to_radians(),
                    self.params.rotate_z.to_radians(),
                ),
                camera_translate: glam::Vec3::new(
                    self.params.position_x,
                    self.params.position_y,
                    self.params.position_z,
                ),
                camera_fov: self.params.fov.to_radians(),
                sample_index: self.sample_count as u32,
                max_recursion_depth: self.params.max_recursion_depth,
                storage_image_index: self.accumulate_storage_image_index,
                instance_params_index,
                materials_index,
                sun_direction: glam::vec2(
                    self.params.sun_direction.x.to_radians(),
                    self.params.sun_direction.y.to_radians(),
                ),
                sun_angle: self.params.sun_angle.to_radians(),
                sun_strength: self.params.sun_strength,
                sun_color: self.params.sun_color,
                sun_enabled: self.params.sun_enabled,
                padding_0: [0; 2],
                padding_1: [0; 1],
            }],
        );

        // ray tracingの実行
        command_buffer.cmd_trace_rays(
            &raygen_shader_sbt_entry,
            &miss_shader_sbt_entry,
            &hit_shader_sbt_entry,
            &vk::StridedDeviceAddressRegionKHR::default(),
            self.params.width,
            self.params.height,
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

    // finalしつつtextureに結果を焼き込む
    fn take_image(&mut self) -> crate::NextImage {
        let image_handles = &self.images[self.current_image_index];
        let fences = [self.final_fences[self.current_image_index].clone()];
        let command_buffer = self.final_command_buffers[self.current_image_index].clone();

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

        command_buffer.cmd_bind_compute_pipeline(&self.final_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.final_compute_pipeline_layout,
            0,
            &[self.descriptor_sets.storage_image.set.clone()],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.final_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[FinalPushConstants {
                sample_count: self.sample_count,
                input_index: self.accumulate_storage_image_index,
                output_index: self.final_storage_image_indices[self.current_image_index],
                l_white: self.params.l_white,
                aperture: self.params.aperture,
                shutter_speed: self.params.shutter_speed,
                iso: self.params.iso,
            }],
        );
        command_buffer.cmd_dispatch((self.params.width + 7) / 8, (self.params.height + 7) / 8, 1);

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
            Some(self.final_fences[self.current_image_index].clone()),
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
