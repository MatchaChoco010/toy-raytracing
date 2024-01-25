use ash::vk;
use bytemuck;
use oidn::{OidnBuffer, OidnDevice, OidnFilter};
use std::time::{Duration, Instant};

use crate::NextImage;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    accumulate_image_index: u32,
    base_color_image_index: u32,
    normal_image_index: u32,
    sample_index: u32,
    camera_rotate: glam::Mat4,
    camera_translate: glam::Vec3,
    camera_fov: f32,
    max_recursion_depth: u32,
    instance_params_index: u32,
    materials_index: u32,
    padding_0: [u32; 1],
    sun_color: glam::Vec3,
    sun_strength: f32,
    sun_direction: glam::Vec2,
    sun_angle: f32,
    sun_enabled: u32,
    sky_width: u32,
    sky_height: u32,
    sky_rotation: f32,
    sky_strength: f32,
    sky_enabled: u32,
    padding_1: [u32; 3],
    sky_buffer_address: u64,
    sky_cdf_row_buffer_address: u64,
    sky_pdf_row_buffer_address: u64,
    sky_cdf_column_buffer_address: u64,
    sky_pdf_column_buffer_address: u64,
    padding_2: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ResolvePushConstants {
    input_index: u32,
    output_index: u32,
    sample_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BeforeDenoisePushConstants {
    color_image_index: u32,
    albedo_image_index: u32,
    normal_image_index: u32,
    padding: [u32; 1],
    color_buffer_address: u64,
    albedo_buffer_address: u64,
    normal_buffer_address: u64,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct AfterDenoisePushConstants {
    output_image_index: u32,
    padding: [u32; 1],
    output_buffer_address: u64,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FinalPushConstants {
    input_index: u32,
    output_index: u32,
    l_white: f32,
    aperture: f32,
    shutter_speed: f32,
    iso: f32,
    enable_tone_mapping: u32,
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

    sampler: ashtray::SamplerHandle,
    accumulate_image: ashtray::utils::ImageHandles,
    base_color_image: ashtray::utils::ImageHandles,
    normal_image: ashtray::utils::ImageHandles,
    resolved_image: ashtray::utils::ImageHandles,
    denoised_image: ashtray::utils::ImageHandles,
    output_images: [ashtray::utils::ImageHandles; 2],

    color_buffer: ashtray::utils::SharedBuffer,
    albedo_buffer: ashtray::utils::SharedBuffer,
    normal_buffer: ashtray::utils::SharedBuffer,
    output_buffer: ashtray::utils::SharedBuffer,

    oidn_device: OidnDevice,
    oidn_filter: OidnFilter,
    oidn_color_buffer: OidnBuffer,
    oidn_albedo_buffer: OidnBuffer,
    oidn_normal_buffer: OidnBuffer,
    oidn_output_buffer: OidnBuffer,

    before_denoise_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    before_denoise_compute_pipeline: ashtray::ComputePipelineHandle,
    after_denoise_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    after_denoise_compute_pipeline: ashtray::ComputePipelineHandle,
    denoise_command_buffer: ashtray::CommandBufferHandle,
    denoise_fence: ashtray::FenceHandle,

    descriptor_sets: ashtray::utils::BindlessDescriptorSets,

    accumulate_image_index: u32,
    base_color_image_index: u32,
    normal_image_index: u32,
    resolved_image_index: u32,
    denoised_image_index: u32,
    output_image_indices: [u32; 2],

    scene_objects: Option<crate::scene::SceneObjects>,

    ray_tracing_pipeline: Option<ashtray::RayTracingPipelineHandle>,
    ray_tracing_pipeline_layout: Option<ashtray::PipelineLayoutHandle>,
    acceleration_structure_descriptor_set:
        Option<ashtray::utils::DescriptorSetAccelerationStructureHandles>,
    shader_binding_table: Option<ashtray::utils::ShaderBindingTable>,
    instance_params_buffer_index: Option<u32>,
    materials_buffer_index: Option<u32>,
    render_command_buffer: ashtray::CommandBufferHandle,
    render_fence: ashtray::FenceHandle,

    resolve_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    resolve_compute_pipeline: ashtray::ComputePipelineHandle,
    resolve_command_buffer: ashtray::CommandBufferHandle,
    resolve_fence: ashtray::FenceHandle,

    output_compute_pipeline_layout: ashtray::PipelineLayoutHandle,
    output_compute_pipeline: ashtray::ComputePipelineHandle,
    output_command_buffers: [ashtray::CommandBufferHandle; 2],
    output_fences: [ashtray::FenceHandle; 2],

    current_image_index: usize,

    sample_count: u32,
    rendering_start_time: Instant,
    rendering_time: Duration,

    need_resolve: bool,
    need_denoise: bool,
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

        // samplerの作成
        let sampler = ashtray::utils::create_sampler(&device);

        // レンダリングに必要なimageの作成
        let accumulate_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let base_color_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let normal_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let resolved_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let denoised_image = ashtray::utils::create_storage_image(
            &device,
            &queue_handles,
            &allocator,
            &transfer_command_buffer,
            width,
            height,
        );
        let output_images = [
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

        // oidn用bufferの確保
        let color_buffer = ashtray::utils::SharedBuffer::new(
            &device,
            width as u64 * height as u64 * 3 * 32,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );
        let albedo_buffer = ashtray::utils::SharedBuffer::new(
            &device,
            width as u64 * height as u64 * 3 * 32,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );
        let normal_buffer = ashtray::utils::SharedBuffer::new(
            &device,
            width as u64 * height as u64 * 3 * 32,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );
        let output_buffer = ashtray::utils::SharedBuffer::new(
            &device,
            width as u64 * height as u64 * 3 * 32,
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        // oidnの初期化
        let oidn_device = OidnDevice::new();
        let oidn_color_buffer = oidn_device.new_buffer(&color_buffer);
        let oidn_albedo_buffer = oidn_device.new_buffer(&albedo_buffer);
        let oidn_normal_buffer = oidn_device.new_buffer(&normal_buffer);
        let oidn_output_buffer = oidn_device.new_buffer(&output_buffer);
        let mut oidn_filter = oidn_device.new_filter("RT");
        oidn_filter.hdr(true);
        oidn_filter.srgb(false);
        oidn_filter.resize(width, height);
        oidn_filter.color(&oidn_color_buffer);
        oidn_filter.albedo(&oidn_albedo_buffer);
        oidn_filter.normal(&oidn_normal_buffer);
        oidn_filter.output(&oidn_output_buffer);

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
        let render_fence = ashtray::utils::create_signaled_fence(&device);

        // bindlessなdescriptor setsを作成
        let descriptor_sets = ashtray::utils::BindlessDescriptorSets::create(&device);
        let accumulate_image_index = 0;
        descriptor_sets
            .storage_image
            .update(&accumulate_image, accumulate_image_index);
        let base_color_image_index = 1;
        descriptor_sets
            .storage_image
            .update(&base_color_image, base_color_image_index);
        let normal_image_index = 2;
        descriptor_sets
            .storage_image
            .update(&normal_image, normal_image_index);
        let resolved_image_index = 3;
        descriptor_sets
            .storage_image
            .update(&resolved_image, resolved_image_index);
        let denoised_image_index = 4;
        descriptor_sets
            .storage_image
            .update(&denoised_image, denoised_image_index);
        let output_image_indices = [5, 6];
        descriptor_sets
            .storage_image
            .update(&output_images[0], output_image_indices[0]);
        descriptor_sets
            .storage_image
            .update(&output_images[1], output_image_indices[1]);

        // resolveのcompute pipelineを作成
        let resolve_compute_pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[*descriptor_sets.storage_image.layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<ResolvePushConstants>() as u32,
                }]),
        );
        let resolve_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/resolve.comp.spv")[..],
        );
        let resolve_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &resolve_compute_pipeline_layout,
            &resolve_compute_shader_module,
        );
        let resolve_command_pool =
            ashtray::utils::create_compute_command_pool(&device, &queue_handles);
        let resolve_command_buffer =
            ashtray::utils::allocate_command_buffers(&device, &resolve_command_pool, 1)
                .into_iter()
                .next()
                .unwrap();
        let resolve_fence = ashtray::utils::create_signaled_fence(&device);

        // denosiseのcompute pipelineを作成
        let before_denoise_compute_pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[
                    *descriptor_sets.storage_image.layout,
                    *descriptor_sets.storage_buffer.layout,
                ])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<BeforeDenoisePushConstants>() as u32,
                }]),
        );
        let before_denoise_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/before_denoise.comp.spv")[..],
        );
        let before_denoise_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &before_denoise_compute_pipeline_layout,
            &before_denoise_compute_shader_module,
        );
        let after_denoise_compute_pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[
                    *descriptor_sets.storage_image.layout,
                    *descriptor_sets.storage_buffer.layout,
                ])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<AfterDenoisePushConstants>() as u32,
                }]),
        );
        let after_denoise_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/after_denoise.comp.spv")[..],
        );
        let after_denoise_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &after_denoise_compute_pipeline_layout,
            &after_denoise_compute_shader_module,
        );
        let denoise_command_buffer =
            ashtray::utils::allocate_command_buffers(&device, &compute_command_pool, 1)
                .into_iter()
                .next()
                .unwrap();
        let denoise_fence = ashtray::utils::create_fence(&device);

        // outputのcompute pipelineを作成
        let output_compute_pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[*descriptor_sets.storage_image.layout])
                .push_constant_ranges(&[vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::COMPUTE,
                    offset: 0,
                    size: std::mem::size_of::<FinalPushConstants>() as u32,
                }]),
        );
        let output_compute_shader_module = ashtray::utils::create_shader_module(
            &device,
            &include_bytes!("./shaders/spv/output.comp.spv")[..],
        );
        let output_compute_pipeline = ashtray::utils::create_compute_pipeline(
            &device,
            &output_compute_pipeline_layout,
            &output_compute_shader_module,
        );
        let output_command_pool =
            ashtray::utils::create_compute_command_pool(&device, &queue_handles);
        let output_command_buffers: [ashtray::CommandBufferHandle; 2] =
            ashtray::utils::allocate_command_buffers(&device, &output_command_pool, 2)
                .try_into()
                .unwrap();
        let output_fences = [
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

            sampler,
            accumulate_image,
            base_color_image,
            normal_image,
            resolved_image,
            denoised_image,
            output_images,

            color_buffer,
            albedo_buffer,
            normal_buffer,
            output_buffer,

            oidn_device,
            oidn_color_buffer,
            oidn_albedo_buffer,
            oidn_normal_buffer,
            oidn_output_buffer,
            oidn_filter,

            before_denoise_compute_pipeline_layout,
            before_denoise_compute_pipeline,
            after_denoise_compute_pipeline_layout,
            after_denoise_compute_pipeline,
            denoise_command_buffer,
            denoise_fence,

            descriptor_sets,

            accumulate_image_index,
            base_color_image_index,
            normal_image_index,
            resolved_image_index,
            denoised_image_index,
            output_image_indices,

            scene_objects: None,

            ray_tracing_pipeline: None,
            ray_tracing_pipeline_layout: None,
            acceleration_structure_descriptor_set: None,
            shader_binding_table: None,
            instance_params_buffer_index: None,
            materials_buffer_index: None,
            render_command_buffer,
            render_fence,

            resolve_compute_pipeline_layout,
            resolve_compute_pipeline,
            resolve_command_buffer,
            resolve_fence,

            output_compute_pipeline_layout,
            output_compute_pipeline,
            output_command_buffers,
            output_fences,

            current_image_index: 0,

            sample_count: 0,
            rendering_start_time: Instant::now(),
            rendering_time: Duration::from_secs(0),

            need_resolve: false,
            need_denoise: false,
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
        let material_closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/material/closesthit.rchit.spv"),
        );
        let material_anyhit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/material/anyhit.rahit.spv"),
        );
        let material_miss_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/material/miss.rmiss.spv"),
        );
        let shadow_closest_hit_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/closesthit.rchit.spv"),
        );
        let shadow_anyhit_alpha_blend_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/anyhit_alpha_blend.rahit.spv"),
        );
        let shadow_anyhit_alpha_mask_shader_module = ashtray::utils::create_shader_module(
            &self.device,
            include_bytes!("./shaders/spv/shadow/anyhit_alpha_mask.rahit.spv"),
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
                    // material opaque
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(material_closest_hit_shader_module.clone()),
                        any_hit: None,
                        intersection: None,
                    },
                    // material alpha mask
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(material_closest_hit_shader_module.clone()),
                        any_hit: Some(material_anyhit_shader_module),
                        intersection: None,
                    },
                    // material alpha blend
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(material_closest_hit_shader_module),
                        any_hit: None,
                        intersection: None,
                    },
                    // shadow opaque
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(shadow_closest_hit_shader_module.clone()),
                        any_hit: None,
                        intersection: None,
                    },
                    // shadow alpha mask
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(shadow_closest_hit_shader_module.clone()),
                        any_hit: Some(shadow_anyhit_alpha_mask_shader_module),
                        intersection: None,
                    },
                    // shadow alpha blend
                    ashtray::utils::HitShaderModules {
                        closest_hit: Some(shadow_closest_hit_shader_module),
                        any_hit: Some(shadow_anyhit_alpha_blend_shader_module),
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
            self.rendering_start_time = Instant::now();
            self.rendering_time = Duration::from_secs(0);

            self.device.wait_idle();

            // imageの再生性
            self.accumulate_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
            );
            self.base_color_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
            );
            self.normal_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
            );
            self.resolved_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
            );
            self.denoised_image = ashtray::utils::create_storage_image(
                &self.device,
                &self.queue_handles,
                &self.allocator,
                &self.transfer_command_buffer,
                self.params.width,
                self.params.height,
            );
            self.output_images = [
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

            // accumulate bufferのクリア
            let command_buffer = self.render_command_buffer.clone();
            command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
            ashtray::utils::begin_onetime_command_buffer(&command_buffer);

            command_buffer.cmd_clear_color_image(
                &self.accumulate_image.image,
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

            // oidn用bufferの確保
            self.color_buffer = ashtray::utils::SharedBuffer::new(
                &self.device,
                self.params.width as u64 * self.params.height as u64 * 3 * 32,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            );
            self.albedo_buffer = ashtray::utils::SharedBuffer::new(
                &self.device,
                self.params.width as u64 * self.params.height as u64 * 3 * 32,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            );
            self.normal_buffer = ashtray::utils::SharedBuffer::new(
                &self.device,
                self.params.width as u64 * self.params.height as u64 * 3 * 32,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            );
            self.output_buffer = ashtray::utils::SharedBuffer::new(
                &self.device,
                self.params.width as u64 * self.params.height as u64 * 3 * 32,
                vk::BufferUsageFlags::TRANSFER_SRC
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            );

            // oidnのfilterのりサイズ
            self.oidn_color_buffer = self.oidn_device.new_buffer(&self.color_buffer);
            self.oidn_albedo_buffer = self.oidn_device.new_buffer(&self.albedo_buffer);
            self.oidn_normal_buffer = self.oidn_device.new_buffer(&self.normal_buffer);
            self.oidn_output_buffer = self.oidn_device.new_buffer(&self.output_buffer);
            self.oidn_filter
                .resize(self.params.width, self.params.height);
            self.oidn_filter.color(&self.oidn_color_buffer);
            self.oidn_filter.albedo(&self.oidn_albedo_buffer);
            self.oidn_filter.normal(&self.oidn_normal_buffer);
            self.oidn_filter.output(&self.oidn_output_buffer);

            // descriptor setの更新
            let accumulate_image_index = 0;
            self.descriptor_sets
                .storage_image
                .update(&self.accumulate_image, accumulate_image_index);
            let base_color_image_index = 1;
            self.descriptor_sets
                .storage_image
                .update(&self.base_color_image, base_color_image_index);
            let normal_image_index = 2;
            self.descriptor_sets
                .storage_image
                .update(&self.normal_image, normal_image_index);
            let resolved_image_index = 3;
            self.descriptor_sets
                .storage_image
                .update(&self.resolved_image, resolved_image_index);
            let denoised_image_index = 4;
            self.descriptor_sets
                .storage_image
                .update(&self.denoised_image, denoised_image_index);
            let output_image_indices = [5, 6];
            self.descriptor_sets
                .storage_image
                .update(&self.output_images[0], output_image_indices[0]);
            self.descriptor_sets
                .storage_image
                .update(&self.output_images[1], output_image_indices[1]);
        } else if self.params != parameters {
            // そうでなくてdirtyなら蓄積をリセットするコマンドのみを発行する。
            self.params = parameters;
            self.sample_count = 0;
            self.rendering_start_time = Instant::now();
            self.rendering_time = Duration::from_secs(0);

            let command_buffer = self.render_command_buffer.clone();
            command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
            ashtray::utils::begin_onetime_command_buffer(&command_buffer);
            command_buffer.cmd_clear_color_image(
                &self.accumulate_image.image,
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
        } else {
            // display imageのみの更新
            self.params = parameters;
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
        let scene = self.scene_objects.as_ref().unwrap();

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
                accumulate_image_index: self.accumulate_image_index,
                base_color_image_index: self.base_color_image_index,
                normal_image_index: self.normal_image_index,
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
                sky_width: scene.sky_texture_width,
                sky_height: scene.sky_texture_height,
                sky_rotation: self.params.sky_rotation.to_radians(),
                sky_strength: self.params.sky_strength,
                sky_enabled: self.params.sky_enabled,
                sky_buffer_address: scene.sky_texture_buffer.device_address,
                sky_cdf_row_buffer_address: scene.sky_texture_cdf_row_buffer.device_address,
                sky_pdf_row_buffer_address: scene.sky_texture_pdf_row_buffer.device_address,
                sky_cdf_column_buffer_address: scene.sky_texture_cdf_column_buffer.device_address,
                sky_pdf_column_buffer_address: scene.sky_texture_pdf_column_buffer.device_address,
                padding_0: [0; 1],
                padding_1: [0; 3],
                padding_2: [0; 2],
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
        self.device.reset_fences(&[self.render_fence.clone()]);
        self.device.queue_submit(
            self.queue_handles.graphics.queue,
            &[submit_info],
            Some(self.render_fence.clone()),
        );
        self.device
            .wait_fences(&[self.render_fence.clone()], u64::MAX);

        self.sample_count += 1;
        self.rendering_time = self.rendering_start_time.elapsed();

        self.need_resolve = true;
    }

    // render imageのresolveする
    fn resolve(&mut self) {
        if !self.need_resolve {
            return;
        }

        let command_buffer = self.resolve_command_buffer.clone();

        command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
        ashtray::utils::begin_onetime_command_buffer(&command_buffer);

        command_buffer.cmd_bind_compute_pipeline(&self.resolve_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.resolve_compute_pipeline_layout,
            0,
            &[self.descriptor_sets.storage_image.set.clone()],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.resolve_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[ResolvePushConstants {
                sample_count: self.sample_count,
                input_index: self.accumulate_image_index,
                output_index: self.resolved_image_index,
            }],
        );
        command_buffer.cmd_dispatch((self.params.width + 7) / 8, (self.params.height + 7) / 8, 1);
        command_buffer.end_command_buffer();

        self.device.reset_fences(&[self.resolve_fence.clone()]);
        self.device.queue_submit(
            self.queue_handles.compute.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[*command_buffer])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])
                    .wait_semaphores(&[]),
            ),
            Some(self.resolve_fence.clone()),
        );
        self.device
            .wait_fences(&[self.resolve_fence.clone()], u64::MAX);

        self.need_resolve = false;
        if self.params.denoise_every_sample || self.sample_count == self.params.max_sample_count {
            self.need_denoise = true;
        }
    }

    fn denoise(&mut self) {
        if !self.need_denoise {
            return;
        }

        // oidn用のbufferに蓄積画像をコピー
        let command_buffer = self.denoise_command_buffer.clone();
        command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
        ashtray::utils::begin_onetime_command_buffer(&command_buffer);
        command_buffer.cmd_bind_compute_pipeline(&self.before_denoise_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.before_denoise_compute_pipeline_layout,
            0,
            &[
                self.descriptor_sets.storage_image.set.clone(),
                self.descriptor_sets.storage_buffer.set.clone(),
            ],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.before_denoise_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[BeforeDenoisePushConstants {
                color_image_index: self.resolved_image_index,
                albedo_image_index: self.base_color_image_index,
                normal_image_index: self.normal_image_index,
                color_buffer_address: self.color_buffer.device_address,
                albedo_buffer_address: self.albedo_buffer.device_address,
                normal_buffer_address: self.normal_buffer.device_address,
                padding: [0; 1],
            }],
        );
        command_buffer.cmd_dispatch((self.params.width + 7) / 8, (self.params.height + 7) / 8, 1);
        command_buffer.end_command_buffer();
        self.device.reset_fences(&[self.denoise_fence.clone()]);
        self.device.queue_submit(
            self.queue_handles.compute.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[*command_buffer])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COMPUTE_SHADER])
                    .wait_semaphores(&[]),
            ),
            Some(self.denoise_fence.clone()),
        );
        self.device
            .wait_fences(&[self.denoise_fence.clone()], u64::MAX);

        // oidnでdenoise
        self.oidn_filter.execute();

        // oidnの結果をoutput imageにコピー
        let command_buffer = self.denoise_command_buffer.clone();
        command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);
        ashtray::utils::begin_onetime_command_buffer(&command_buffer);
        command_buffer.cmd_bind_compute_pipeline(&self.after_denoise_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.after_denoise_compute_pipeline_layout,
            0,
            &[
                self.descriptor_sets.storage_image.set.clone(),
                self.descriptor_sets.storage_buffer.set.clone(),
            ],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.after_denoise_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[AfterDenoisePushConstants {
                output_image_index: self.denoised_image_index,
                output_buffer_address: self.output_buffer.device_address,
                padding: [0; 1],
            }],
        );
        command_buffer.cmd_dispatch((self.params.width + 7) / 8, (self.params.height + 7) / 8, 1);
        command_buffer.end_command_buffer();
        self.device.reset_fences(&[self.denoise_fence.clone()]);
        self.device.queue_submit(
            self.queue_handles.compute.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[*command_buffer])
                    .wait_dst_stage_mask(&[])
                    .wait_semaphores(&[]),
            ),
            Some(self.denoise_fence.clone()),
        );
        self.device
            .wait_fences(&[self.denoise_fence.clone()], u64::MAX);

        self.need_denoise = false;
    }

    // output textureに結果を焼き込む
    fn output_image(&mut self) -> crate::NextImage {
        let input_image_index = match self.params.display_image {
            crate::DisplayImage::BaseColor => self.base_color_image_index,
            crate::DisplayImage::Normal => self.normal_image_index,
            crate::DisplayImage::Resolved => self.resolved_image_index,
            crate::DisplayImage::Final => {
                if self.params.denoise_every_sample
                    || self.sample_count == self.params.max_sample_count
                {
                    self.denoised_image_index
                } else {
                    self.resolved_image_index
                }
            }
        };
        let enable_tone_mapping = if self.params.display_image == crate::DisplayImage::Final
            || self.params.display_image == crate::DisplayImage::Resolved
        {
            1
        } else {
            0
        };
        let image_handles = &self.output_images[self.current_image_index];
        let fences = [self.output_fences[self.current_image_index].clone()];
        let command_buffer = self.output_command_buffers[self.current_image_index].clone();

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

        command_buffer.cmd_bind_compute_pipeline(&self.output_compute_pipeline);
        command_buffer.cmd_bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.output_compute_pipeline_layout,
            0,
            &[self.descriptor_sets.storage_image.set.clone()],
            &[],
        );
        command_buffer.cmd_push_constants(
            &self.output_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            &[FinalPushConstants {
                input_index: input_image_index,
                output_index: self.output_image_indices[self.current_image_index],
                l_white: self.params.l_white,
                aperture: self.params.aperture,
                shutter_speed: self.params.shutter_speed,
                iso: self.params.iso,
                enable_tone_mapping,
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
            Some(self.output_fences[self.current_image_index].clone()),
        );

        let image_view = image_handles.image_view.clone();
        let sampler = self.sampler.clone();
        let sample_count = self.sample_count;

        self.current_image_index = (self.current_image_index + 1) % 2;

        NextImage {
            image_view,
            sampler,
            sample_count,
            rendering_time: self.rendering_time,
        }
    }

    pub fn render(&mut self, parameters: crate::Parameters) -> NextImage {
        self.set_parameters(parameters);
        self.ray_trace();
        self.resolve();
        self.denoise();
        self.output_image()
    }
}
