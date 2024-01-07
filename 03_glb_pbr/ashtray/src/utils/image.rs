use crate::utils::*;
use ash::vk;

/// Imageのハンドルをまとめた構造体
pub struct ImageHandles {
    /// ImageHandle
    pub image: crate::ImageHandle,
    /// AllocationHandle
    pub allocation: crate::AllocationHandle,
    /// ImageViewHandle
    pub image_view: crate::ImageViewHandle,
}

/// storage imageを作成する関数
pub fn create_storage_image(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    allocator: &crate::AllocatorHandle,
    image_transfer_command_buffer: &crate::CommandBufferHandle,
    width: u32,
    height: u32,
) -> ImageHandles {
    // imageの生成
    let image_create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = device.create_image(&image_create_info);

    // imageのメモリ確保
    let image_memory_requirement = image.get_image_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "storage_image",
        requirements: image_memory_requirement,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // imageとメモリのバインド
    image.bind_image_memory(allocation.memory(), allocation.offset());

    // image_viewの作成
    let image_view_create_info = vk::ImageViewCreateInfo::builder()
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .components(
            vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY)
                .build(),
        )
        .subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )
        .image(*image);
    let image_view = device.create_image_view(&image_view_create_info);

    {
        let fence = create_fence(device);
        begin_onetime_command_buffer(image_transfer_command_buffer);
        cmd_image_barriers(
            image_transfer_command_buffer,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::UNDEFINED,
            vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::GENERAL,
            &image,
        );
        image_transfer_command_buffer.end_command_buffer();
        device.queue_submit(
            queue_handles.transfer.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[**image_transfer_command_buffer])
                    .wait_dst_stage_mask(&[])
                    .wait_semaphores(&[]),
            ),
            Some(fence.clone()),
        );
        device.wait_fences(&[fence], u64::MAX);
    }

    ImageHandles {
        image,
        allocation,
        image_view,
    }
}

/// storage imageを作成する関数
pub fn create_shader_readonly_image(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    allocator: &crate::AllocatorHandle,
    image_transfer_command_buffer: &crate::CommandBufferHandle,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
) -> ImageHandles {
    // imageの生成
    let image_create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = device.create_image(&image_create_info);

    // imageのメモリ確保
    let image_memory_requirement = image.get_image_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "storage_image",
        requirements: image_memory_requirement,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // imageとメモリのバインド
    image.bind_image_memory(allocation.memory(), allocation.offset());

    // image_viewの作成
    let image_view_create_info = vk::ImageViewCreateInfo::builder()
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(
            vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY)
                .build(),
        )
        .subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )
        .image(*image);
    let image_view = device.create_image_view(&image_view_create_info);

    // imageのlayoutをshader readonly optimalに変更
    {
        let fence = create_fence(device);
        begin_onetime_command_buffer(image_transfer_command_buffer);
        cmd_image_barriers(
            image_transfer_command_buffer,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::UNDEFINED,
            vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            &image,
        );
        image_transfer_command_buffer.end_command_buffer();
        device.queue_submit(
            queue_handles.transfer.queue,
            std::slice::from_ref(
                &vk::SubmitInfo::builder()
                    .command_buffers(&[**image_transfer_command_buffer])
                    .wait_dst_stage_mask(&[])
                    .wait_semaphores(&[]),
            ),
            Some(fence.clone()),
        );
        device.wait_fences(&[fence], u64::MAX);
    }

    ImageHandles {
        image,
        allocation,
        image_view,
    }
}

/// samplerをNEARESTで作成するヘルパー関数
pub fn create_sampler(device: &crate::DeviceHandle) -> crate::SamplerHandle {
    let create_info = vk::SamplerCreateInfo::builder()
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .mag_filter(vk::Filter::NEAREST)
        .min_filter(vk::Filter::NEAREST)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST);
    device.create_sampler(&create_info)
}

/// BindlessなDescriptorSetをまとめた構造体
pub struct BindlessDescriptorSets {
    /// uniform bufferのdescriptor set
    pub uniform_buffer: DescriptorSetUniformBufferHandles,
    /// combined image samplerのdescriptor set
    pub combined_image_sampler: DescriptorSetCombinedImageSamplerHandles,
    /// storage bufferのdescriptor set
    pub storage_buffer: DescriptorSetStorageBufferHandles,
    /// storage imageのdescriptor set
    pub storage_image: DescriptorSetStorageImageHandles,
}
impl BindlessDescriptorSets {
    /// BindlessなDescriptorSetをまとめた構造体を作成する
    pub fn create(device: &crate::DeviceHandle) -> Self {
        let uniform_buffer = DescriptorSetUniformBufferHandles::create(device);
        let combined_image_sampler = DescriptorSetCombinedImageSamplerHandles::create(device);
        let storage_buffer = DescriptorSetStorageBufferHandles::create(device);
        let storage_image = DescriptorSetStorageImageHandles::create(device);
        Self {
            uniform_buffer,
            combined_image_sampler,
            storage_buffer,
            storage_image,
        }
    }
}
