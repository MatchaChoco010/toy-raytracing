use ash::vk;
use bytemuck;

use crate::NextImage;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    camera_rotate: glam::Mat4,
    camera_translate: glam::Vec3,
    seed: u32,
    max_recursion_depth: u32,
    l_white: f32,
    storage_image_index: u32,
    instance_params_index: u32,
    materials_index: u32,
    padding1: u32,
    padding2: u64,
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
    l_white: f32,
    max_recursion_depth: u32,

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
                    size: std::mem::size_of::<u32>() as u32 * 4,
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
            width,
            height,
            max_sample_count: 256,
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            l_white: 1.0,
            max_recursion_depth: 1,

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
        let miss_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/miss.rmiss.spv"),
        );
        let closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/closesthit.rchit.spv"),
        );
        let (ray_tracing_pipeline, pipeline_layout) = ashtray::utils::create_ray_tracing_pipelines(
            &self.device,
            &[raygen_shader_module],
            &[miss_shader_module],
            &[ashtray::utils::HitShaderModules {
                closest_hit: closest_hit_shader_module,
                any_hit: None,
                intersection: None,
            }],
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

        self.scene_objects = Some(scene_objects);
        self.ray_tracing_pipeline = Some(ray_tracing_pipeline);
        self.ray_tracing_pipeline_layout = Some(pipeline_layout);
        self.acceleration_structure_descriptor_set = Some(acceleration_structure_descriptor_set);
        self.shader_binding_table = Some(shader_binding_table);
        self.instance_params_buffer_index = Some(instance_params_buffer_index);
        self.materials_buffer_index = Some(materials_buffer_index);
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
            self.l_white = parameters.l_white;
            self.max_recursion_depth = parameters.max_recursion_depth;

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

            self.descriptor_sets
                .storage_image
                .update(&self.storage_image, self.accumulate_storage_image_index);
            self.descriptor_sets
                .storage_image
                .update(&self.images[0], self.final_storage_image_indices[0]);
            self.descriptor_sets
                .storage_image
                .update(&self.images[1], self.final_storage_image_indices[1]);
        } else if self.max_sample_count != parameters.max_sample_count
            || self.rotate_x != parameters.rotate_x
            || self.rotate_y != parameters.rotate_y
            || self.rotate_z != parameters.rotate_z
            || self.position_x != parameters.position_x
            || self.position_y != parameters.position_y
            || self.position_z != parameters.position_z
            || self.l_white != parameters.l_white
            || self.max_recursion_depth != parameters.max_recursion_depth
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
            self.l_white = parameters.l_white;
            self.max_recursion_depth = parameters.max_recursion_depth;

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
                max_recursion_depth: self.max_recursion_depth,
                l_white: self.l_white,
                storage_image_index: self.accumulate_storage_image_index,
                instance_params_index,
                materials_index,
                padding1: 0,
                padding2: 0,
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

    // finalしつつtextureに結果を焼き込む
    fn take_image(&mut self) -> crate::NextImage {
        let image_handles = &self.images[self.current_image_index];
        // let fences = [self.final_fences[self.current_image_index].clone()];
        let command_buffer = self.final_command_buffers[self.current_image_index].clone();

        // self.device.wait_fences(&fences, u64::MAX);
        // self.device.reset_fences(&fences);

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
            &[
                self.sample_count,
                self.accumulate_storage_image_index,
                self.final_storage_image_indices[self.current_image_index],
                self.l_white.to_bits(),
            ],
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
        let fences = [self.final_fences[self.current_image_index].clone()];
        self.device.wait_fences(&fences, u64::MAX);
        self.device.reset_fences(&fences);
        self.set_parameters(parameters);
        self.ray_trace();
        self.take_image()
    }
}
