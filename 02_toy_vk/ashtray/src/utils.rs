use ash::{
    extensions::khr::{
        AccelerationStructure, DeferredHostOperations, RayTracingPipeline, Swapchain,
        Synchronization2, TimelineSemaphore,
    },
    vk,
};
use std::collections::HashSet;
use std::ffi::CString;

/// 必要なdevice拡張のリストを取得する関数
pub fn get_required_device_extensions(required_device_extensions: &[CString]) -> Vec<CString> {
    let mut required_device_extensions = required_device_extensions.to_vec();
    required_device_extensions.append(&mut vec![
        Swapchain::name().to_owned(),
        Synchronization2::name().to_owned(),
        TimelineSemaphore::name().to_owned(),
        RayTracingPipeline::name().to_owned(),
        AccelerationStructure::name().to_owned(),
        DeferredHostOperations::name().to_owned(),
    ]);
    required_device_extensions
}

/// 適当なphysical deviceを選択する関数
pub fn select_physical_device(
    instance: &crate::InstanceHandle,
    surface: &crate::SurfaceHandle,
    required_device_extensions: &[CString],
) -> vk::PhysicalDevice {
    let physical_devices = instance.enumerate_physical_devices();

    // GraphicsとTransfer、Compute、PresentをサポートしているQueueFamilyがある &&
    // 必要なデバイス拡張機能に対応している &&
    // swapchainに対応したフォーマット / presentationモードが一つ以上ある &&
    // 必要なdevice featuresに対応しているような
    // physical deviceを選択する
    let physical_device = physical_devices.into_iter().find(|physical_device| {
        // QueueFamilyの各種Queue対応の確認
        let mut graphics_index = None;
        let mut transfer_index = None;
        let mut compute_index = None;
        let mut present_index = None;
        let queue_families = instance.get_physical_device_queue_family_properties(*physical_device);
        for (i, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics_index = Some(i);
            }
            if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                transfer_index = Some(i);
            }
            if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                compute_index = Some(i);
            }
            let present_support =
                surface.get_physical_device_surface_support(*physical_device, i as u32);
            if present_support {
                present_index = Some(i);
            }
            if graphics_index.is_some()
                && transfer_index.is_some()
                && compute_index.is_some()
                && present_index.is_some()
            {
                break;
            }
        }
        let is_queue_family_supported = graphics_index.is_some()
            && transfer_index.is_some()
            && compute_index.is_some()
            && present_index.is_some();

        // デバイス拡張の確認
        let device_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(*physical_device)
                .unwrap()
        };
        let mut device_extensions_name = vec![];
        for device_extension in device_extensions {
            let name = unsafe {
                std::ffi::CStr::from_ptr(device_extension.extension_name.as_ptr()).to_owned()
            };
            device_extensions_name.push(name);
        }
        let mut required_extensions = HashSet::new();
        for extension in required_device_extensions.iter() {
            required_extensions.insert(extension.to_owned());
        }
        for extension_name in device_extensions_name {
            required_extensions.remove(&extension_name);
        }
        let is_device_extension_supported = required_extensions.is_empty();

        // swapchainのサポート確認
        let surface_formats = surface.get_physical_device_surface_formats(*physical_device);
        let surface_present_modes =
            surface.get_physical_device_surface_present_modes(*physical_device);
        let is_swapchain_supported =
            !surface_formats.is_empty() && !surface_present_modes.is_empty();

        // featureのサポート確認
        let mut supported_feature_vulkan_12 = vk::PhysicalDeviceVulkan12Features::builder().build();
        let mut supported_feature_vulkan_13 = vk::PhysicalDeviceVulkan13Features::builder().build();
        let mut physical_device_raytracing_pipeline_features_khr =
            vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
                .ray_tracing_pipeline(true)
                .build();
        let mut physical_device_acceleration_structure_feature_khr =
            vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
                .acceleration_structure(true)
                .build();
        let mut supported_feature = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut supported_feature_vulkan_12)
            .push_next(&mut supported_feature_vulkan_13)
            .push_next(&mut physical_device_raytracing_pipeline_features_khr)
            .push_next(&mut physical_device_acceleration_structure_feature_khr)
            .build();
        unsafe { instance.get_physical_device_features2(*physical_device, &mut supported_feature) };
        let is_supported_device_features = supported_feature.features.shader_int64 == vk::TRUE
            && supported_feature_vulkan_12.timeline_semaphore == vk::TRUE
            && supported_feature_vulkan_12.scalar_block_layout == vk::TRUE
            && supported_feature_vulkan_13.synchronization2 == vk::TRUE;

        is_queue_family_supported
            && is_swapchain_supported
            && is_device_extension_supported
            && is_supported_device_features
    });

    let physical_device = physical_device.expect("No suitable physical device");

    physical_device
}

/// 各種Queueのindexを格納する構造体
pub struct QueueIndices {
    pub graphics_index: u32,
    pub transfer_index: u32,
    pub compute_index: u32,
    pub present_index: u32,
}

/// 各種Queueのindexを取得する関数
pub fn get_queue_indices(
    instance: &crate::InstanceHandle,
    surface: &crate::SurfaceHandle,
    physical_device: vk::PhysicalDevice,
) -> QueueIndices {
    // get queue index
    let mut graphics_index = None;
    let mut transfer_index = None;
    let mut compute_index = None;
    let mut present_index = None;
    let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
    for (i, queue_family) in queue_families.iter().enumerate() {
        if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            if graphics_index.is_none() {
                graphics_index = Some(i);
                continue;
            }
            // graphics_index = Some(i);
        }
        if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
            if transfer_index.is_none() {
                transfer_index = Some(i);
                continue;
            }
            // transfer_index = Some(i);
        }
        if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
            compute_index = Some(i);
        }
        let present_support =
            surface.get_physical_device_surface_support(physical_device, i as u32);
        if present_support {
            present_index = Some(i);
        }
    }
    QueueIndices {
        graphics_index: graphics_index.unwrap() as u32,
        transfer_index: transfer_index.unwrap() as u32,
        compute_index: compute_index.unwrap() as u32,
        present_index: present_index.unwrap() as u32,
    }
}

/// deviceを作成する関数
pub fn create_device(
    instance: &crate::InstanceHandle,
    physical_device: vk::PhysicalDevice,
    queue_indices: &QueueIndices,
    required_device_extensions: &[CString],
) -> crate::DeviceHandle {
    // queue create info
    let mut unique_queue_families = HashSet::new();
    unique_queue_families.insert(queue_indices.graphics_index);
    unique_queue_families.insert(queue_indices.transfer_index);
    unique_queue_families.insert(queue_indices.compute_index);
    unique_queue_families.insert(queue_indices.present_index);
    let queue_priorities = [1.0_f32];
    let mut queue_create_infos = vec![];
    for queue_family in unique_queue_families {
        let queue_create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family as u32)
            .queue_priorities(&queue_priorities)
            .build();
        queue_create_infos.push(queue_create_info);
    }

    // physical device features
    let mut physical_device_features = vk::PhysicalDeviceFeatures::builder().build();
    physical_device_features.shader_int64 = vk::TRUE;
    let mut physical_device_vulkan_12_features = vk::PhysicalDeviceVulkan12Features::builder()
        .timeline_semaphore(true)
        .buffer_device_address(true)
        .scalar_block_layout(true)
        .build();
    let mut physical_device_vulkan_13_features = vk::PhysicalDeviceVulkan13Features::builder()
        .synchronization2(true)
        .build();
    let mut physical_device_raytracing_pipeline_features_khr =
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
            .ray_tracing_pipeline(true)
            .build();
    let mut physical_device_acceleration_structure_feature_khr =
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
            .acceleration_structure(true)
            .build();
    // enable extension names
    let enable_extension_names = required_device_extensions
        .iter()
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();
    let enable_extension_names = enable_extension_names
        .iter()
        .map(|s| s.as_ptr())
        .collect::<Vec<_>>();

    // device create info
    let device_create_info = vk::DeviceCreateInfo::builder()
        .push_next(&mut physical_device_vulkan_12_features)
        .push_next(&mut physical_device_vulkan_13_features)
        .push_next(&mut physical_device_raytracing_pipeline_features_khr)
        .push_next(&mut physical_device_acceleration_structure_feature_khr)
        .queue_create_infos(&queue_create_infos)
        .enabled_features(&physical_device_features)
        .enabled_extension_names(&enable_extension_names);

    // create device
    instance.create_device(physical_device, &device_create_info)
}

/// Queueとそのindexを格納する構造体
#[derive(Debug, Clone)]
pub struct QueueHandle {
    pub queue: vk::Queue,
    pub family_index: u32,
    pub index: u32,
}

/// 各種QueueのQueueHandleを格納する構造体
#[derive(Debug, Clone)]
pub struct QueueHandles {
    pub graphics: QueueHandle,
    pub transfer: QueueHandle,
    pub compute: QueueHandle,
    pub present: QueueHandle,
}

/// 各種QueueのQueueHandleを取得する関数
pub fn get_queue_handles(
    device: &crate::DeviceHandle,
    queue_indices: &QueueIndices,
) -> QueueHandles {
    // get device queue
    let graphics_queue = device.get_device_queue(queue_indices.graphics_index, 0);
    let transfer_queue = device.get_device_queue(queue_indices.transfer_index, 0);
    let compute_queue = device.get_device_queue(queue_indices.compute_index, 0);
    let present_queue = device.get_device_queue(queue_indices.present_index, 0);

    QueueHandles {
        graphics: QueueHandle {
            queue: graphics_queue,
            family_index: queue_indices.graphics_index,
            index: 0,
        },
        transfer: QueueHandle {
            queue: transfer_queue,
            family_index: queue_indices.transfer_index,
            index: 0,
        },
        compute: QueueHandle {
            queue: compute_queue,
            family_index: queue_indices.compute_index,
            index: 0,
        },
        present: QueueHandle {
            queue: present_queue,
            family_index: queue_indices.present_index,
            index: 0,
        },
    }
}

/// graphics用のcommand poolを作成する関数
pub fn create_graphics_command_pool(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
) -> crate::CommandPoolHandle {
    let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_handles.graphics.family_index)
        .build();
    device.create_command_pool(&command_pool_create_info)
}

/// transfer用のcommand poolを作成する関数
pub fn create_transfer_command_pool(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
) -> crate::CommandPoolHandle {
    let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_handles.transfer.family_index)
        .build();
    device.create_command_pool(&command_pool_create_info)
}

/// compute用のcommand poolを作成する関数
pub fn create_compute_command_pool(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
) -> crate::CommandPoolHandle {
    let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_handles.compute.family_index)
        .build();
    device.create_command_pool(&command_pool_create_info)
}

/// swapchainの関連オブジェクト
pub struct SwapchainObjects {
    pub swapchain: crate::SwapchainHandle,
    pub swapchain_images: Vec<vk::Image>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
}

/// swapchainを作成する関数
pub fn create_swapchain_objects(
    width: u32,
    height: u32,
    surface: &crate::SurfaceHandle,
    physical_device: vk::PhysicalDevice,
    device: &crate::DeviceHandle,
) -> SwapchainObjects {
    let surface_capabilities = surface.get_physical_device_surface_capabilities(physical_device);
    let surface_formats = surface.get_physical_device_surface_formats(physical_device);
    let surface_present_modes = surface.get_physical_device_surface_present_modes(physical_device);

    // surfaceのformatの選択
    let surface_format = surface_formats
        .iter()
        .find(|surface_format| {
            surface_format.format == vk::Format::B8G8R8A8_SRGB
                && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(&surface_formats[0])
        .clone();

    // surfaceのpresent modeの選択
    let surface_present_mode = surface_present_modes
        .iter()
        .find(|&&present_mode| present_mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(&vk::PresentModeKHR::FIFO);

    // surfaceのextentの選択
    let surface_extent = if surface_capabilities.current_extent.width != u32::MAX {
        surface_capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(
                surface_capabilities.min_image_extent.width,
                surface_capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                surface_capabilities.min_image_extent.height,
                surface_capabilities.max_image_extent.height,
            ),
        }
    };

    // image countの選択
    let image_count = surface_capabilities.min_image_count + 1;
    let image_count = if surface_capabilities.max_image_count != 0 {
        image_count.min(surface_capabilities.max_image_count)
    } else {
        image_count
    };

    // swapchainの作成
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
        .surface(**surface)
        .min_image_count(image_count)
        .image_color_space(surface_format.color_space)
        .image_format(surface_format.format)
        .image_extent(surface_extent)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(*surface_present_mode)
        .image_array_layers(1)
        .clipped(true);
    let swapchain = device.create_swapchain(surface, &swapchain_create_info);

    // swapchainのimageの取得
    let swapchain_images = device.get_swapchain_images(&swapchain);

    SwapchainObjects {
        swapchain,
        swapchain_images,
        format: surface_format.format,
        extent: surface_extent,
    }
}

/// allocatorを作成する関数
pub fn create_allocator(
    instance: &crate::InstanceHandle,
    physical_device: vk::PhysicalDevice,
    device: &crate::DeviceHandle,
) -> crate::AllocatorHandle {
    let allocator_create_desc = gpu_allocator::vulkan::AllocatorCreateDesc {
        instance: unsafe { instance.instance_raw() },
        device: unsafe { device.device_raw() },
        physical_device,
        debug_settings: Default::default(),
        buffer_device_address: true,
        allocation_sizes: Default::default(),
    };
    device.create_allocator(&allocator_create_desc)
}

/// 指定したcommand_poolからPrimaryレベルのcommand bufferをallocateする関数
pub fn allocate_command_buffers(
    device: &crate::DeviceHandle,
    command_pool: &crate::CommandPoolHandle,
    count: u32,
) -> Vec<crate::CommandBufferHandle> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(**command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count);
    let command_buffers =
        device.allocate_command_buffers(&command_pool, &command_buffer_allocate_info);
    command_buffers
}

/// command bufferをリセットしてone time submit用にbeginする関数
pub fn begin_onetime_command_buffer(command_buffer: &crate::CommandBufferHandle) {
    // reset command buffer
    command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);

    // begin command buffer
    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    command_buffer.begin_command_buffer(&begin_info);
}

pub fn cmd_image_barriers(
    command_buffer: &crate::CommandBufferHandle,
    src_stage_mask: vk::PipelineStageFlags2,
    src_access_mask: vk::AccessFlags2,
    old_layout: vk::ImageLayout,
    dst_stage_mask: vk::PipelineStageFlags2,
    dst_access_mask: vk::AccessFlags2,
    new_layout: vk::ImageLayout,
    image: &vk::Image,
) {
    // 画像レイアウト変更のコマンドのレコード
    command_buffer.cmd_pipeline_barrier2(
        &vk::DependencyInfoKHR::builder().image_memory_barriers(std::slice::from_ref(
            &vk::ImageMemoryBarrier2::builder()
                .src_stage_mask(src_stage_mask)
                .src_access_mask(src_access_mask)
                .old_layout(old_layout)
                .dst_stage_mask(dst_stage_mask)
                .dst_access_mask(dst_access_mask)
                .new_layout(new_layout)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .image(*image),
        )),
    );
}

pub struct ImageHandles {
    pub image: crate::ImageHandle,
    pub allocation: crate::AllocationHandle,
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
    let image_view = image.create_image_view(&image_view_create_info);

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
    let image_view = image.create_image_view(&image_view_create_info);

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

#[derive(Clone)]
pub struct BufferObjects {
    pub buffer: crate::BufferHandle,
    pub allocation: crate::AllocationHandle,
    pub device_address: u64,
}

pub fn create_host_buffer(
    device: &crate::DeviceHandle,
    allocator: &crate::AllocatorHandle,
    buffer_size: u64,
    usage: vk::BufferUsageFlags,
) -> BufferObjects {
    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(usage);
    let buffer = device.create_buffer(&buffer_create_info);

    // bufferのメモリ確保
    let buffer_memory_requirement = buffer.get_buffer_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "host buffer",
        requirements: buffer_memory_requirement,
        location: gpu_allocator::MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // bufferとメモリのバインド
    buffer.bind_buffer_memory(allocation.memory(), allocation.offset());

    // device addressの取得
    let device_address =
        device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}

pub fn create_host_buffer_with_data<T>(
    device: &crate::DeviceHandle,
    allocator: &crate::AllocatorHandle,
    data: &[T],
    usage: vk::BufferUsageFlags,
) -> BufferObjects {
    let buffer_size = (std::mem::size_of::<T>() * data.len()) as u64;

    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(usage);
    let buffer = device.create_buffer(&buffer_create_info);

    // bufferのメモリ確保
    let buffer_memory_requirement = buffer.get_buffer_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "host buffer",
        requirements: buffer_memory_requirement,
        location: gpu_allocator::MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // bufferとメモリのバインド
    buffer.bind_buffer_memory(allocation.memory(), allocation.offset());

    // データのコピー
    let ptr = allocation.mapped_ptr().unwrap().as_ptr();
    unsafe {
        std::ptr::copy_nonoverlapping(
            data.as_ptr() as *const u8,
            ptr as *mut u8,
            buffer_size as usize,
        )
    };

    // device addressの取得
    let device_address =
        device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}

pub fn create_device_local_buffer(
    device: &crate::DeviceHandle,
    allocator: &crate::AllocatorHandle,
    buffer_size: u64,
    usage: vk::BufferUsageFlags,
) -> BufferObjects {
    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(usage);
    let buffer = device.create_buffer(&buffer_create_info);

    // bufferのメモリ確保
    let buffer_memory_requirement = buffer.get_buffer_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "device local buffer",
        requirements: buffer_memory_requirement,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // bufferとメモリのバインド
    buffer.bind_buffer_memory(allocation.memory(), allocation.offset());

    // device addressの取得
    let device_address =
        device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}

pub fn create_device_local_buffer_with_data<T>(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    transfer_command_pool: &crate::CommandPoolHandle,
    allocator: &crate::AllocatorHandle,
    data: &[T],
    usage: vk::BufferUsageFlags,
) -> BufferObjects {
    let buffer_size = (std::mem::size_of::<T>() * data.len()) as u64;

    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(usage | vk::BufferUsageFlags::TRANSFER_DST);
    let buffer = device.create_buffer(&buffer_create_info);

    // bufferのメモリ確保
    let buffer_memory_requirement = buffer.get_buffer_memory_requirements();
    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "device local buffer",
        requirements: buffer_memory_requirement,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // bufferとメモリのバインド
    buffer.bind_buffer_memory(allocation.memory(), allocation.offset());

    // device addressの取得
    let device_address =
        device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

    // staging bufferの作成
    let staging_buffer = create_host_buffer_with_data(
        device,
        allocator,
        data,
        usage | vk::BufferUsageFlags::TRANSFER_SRC,
    );

    // bufferのコピー
    let fence = create_fence(device);
    let command_buffer = &allocate_command_buffers(device, transfer_command_pool, 1)[0];
    begin_onetime_command_buffer(&command_buffer);
    command_buffer.cmd_copy_buffer(
        &staging_buffer.buffer,
        &buffer,
        std::slice::from_ref(&vk::BufferCopy::builder().size(buffer_size)),
    );
    command_buffer.end_command_buffer();
    device.queue_submit(
        queue_handles.transfer.queue,
        std::slice::from_ref(
            &vk::SubmitInfo::builder()
                .command_buffers(&[**command_buffer])
                .wait_dst_stage_mask(&[])
                .wait_semaphores(&[]),
        ),
        Some(fence.clone()),
    );
    device.wait_fences(&[fence], u64::MAX);

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}

pub fn create_shader_module(
    device: &crate::DeviceHandle,
    bytes: &[u8],
) -> crate::ShaderModuleHandle {
    let words = bytes
        .chunks_exact(4)
        .map(|x| x.try_into().unwrap())
        .map(match bytes[0] {
            0x03 => u32::from_le_bytes,
            0x07 => u32::from_be_bytes,
            _ => panic!("Unknown endianness"),
        })
        .collect::<Vec<u32>>();
    let create_info = vk::ShaderModuleCreateInfo::builder().code(&words);
    device.create_shader_module(&create_info)
}

pub fn create_compute_pipeline(
    device: &crate::DeviceHandle,
    pipeline_layout: &crate::PipelineLayoutHandle,
    shader_module: &crate::ShaderModuleHandle,
) -> crate::ComputePipelineHandle {
    let entry_name = std::ffi::CString::new("main").unwrap();
    let create_info = vk::ComputePipelineCreateInfo::builder()
        .stage(
            *vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(**shader_module)
                .name(entry_name.as_c_str()),
        )
        .layout(**pipeline_layout);
    device
        .create_compute_pipelines(
            vk::PipelineCache::null(),
            pipeline_layout,
            std::slice::from_ref(&create_info),
        )
        .into_iter()
        .next()
        .unwrap()
}

pub fn create_fence(device: &crate::DeviceHandle) -> crate::FenceHandle {
    let create_info = vk::FenceCreateInfo::builder();
    device.create_fence(&create_info)
}

pub fn create_signaled_fence(device: &crate::DeviceHandle) -> crate::FenceHandle {
    let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    device.create_fence(&create_info)
}

#[derive(Clone)]
pub struct BlasObjects {
    pub blas: crate::AccelerationStructureHandle,
    pub blas_buffer: BufferObjects,
    pub vertex_buffer: BufferObjects,
    pub index_buffer: BufferObjects,
}

pub fn cerate_blas<T>(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    compute_command_pool: &crate::CommandPoolHandle,
    allocator: &crate::AllocatorHandle,
    vertices: &[T],
    indices: &[u32],
) -> BlasObjects {
    let vertex_buffer = create_host_buffer_with_data(
        &device,
        &allocator,
        &vertices,
        vk::BufferUsageFlags::VERTEX_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
    );
    let index_buffer = create_host_buffer_with_data(
        &device,
        &allocator,
        &indices,
        vk::BufferUsageFlags::INDEX_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
    );

    // geometryを作成
    let geometry_triangle_date = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer.device_address,
        })
        // .vertex_stride(std::mem::size_of::<T>() as u64)
        .max_vertex(indices.len() as u32 - 1)
        .vertex_stride(std::mem::size_of::<T>() as u64)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer.device_address,
        });
    let geometry = vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
        .geometry(vk::AccelerationStructureGeometryDataKHR {
            triangles: *geometry_triangle_date,
        })
        .flags(vk::GeometryFlagsKHR::OPAQUE);

    // build geometry infoを作成
    let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
        .geometries(std::slice::from_ref(&geometry))
        .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
        .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
        .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
        .src_acceleration_structure(vk::AccelerationStructureKHR::null());

    // 必要なバッファサイズを取得
    let primitive_count = (indices.len() / 3) as u32;
    let build_size_info = device.get_acceleration_structure_build_sizes(
        vk::AccelerationStructureBuildTypeKHR::DEVICE,
        &build_geometry_info,
        &[primitive_count],
    );

    // バッファを確保
    let blas_buffer = create_device_local_buffer(
        &device,
        &allocator,
        build_size_info.acceleration_structure_size,
        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    // blasの作成
    let blas = device.create_acceleration_structure(
        &vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(*blas_buffer.buffer)
            .size(build_size_info.acceleration_structure_size)
            .offset(0)
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL),
    );

    // scratch bufferの作成
    let scratch_buffer = create_device_local_buffer(
        &device,
        &allocator,
        build_size_info.build_scratch_size,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    // acceleration structureのビルドコマンド実行
    {
        // build用にbuild geometry infoを作成
        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .geometries(std::slice::from_ref(&geometry))
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .src_acceleration_structure(vk::AccelerationStructureKHR::null())
            .dst_acceleration_structure(*blas)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer.device_address,
            });
        // build range infoを作成
        let acceleration_structure_build_range_info =
            vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .primitive_count(primitive_count)
                .primitive_offset(0)
                .first_vertex(0)
                .transform_offset(0);

        // コマンドバッファの開始
        let command_buffer = {
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(**compute_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffers = device
                .allocate_command_buffers(&compute_command_pool, &command_buffer_allocate_info);
            command_buffers.into_iter().next().unwrap()
        };
        begin_onetime_command_buffer(&command_buffer);

        // コマンドのレコード
        // acceleration structureのビルド
        command_buffer.cmd_build_acceleration_structures(
            std::slice::from_ref(&build_geometry_info),
            &[std::slice::from_ref(
                &acceleration_structure_build_range_info,
            )],
        );
        // メモリバリア
        let barrier = vk::MemoryBarrier2KHR::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR)
            .src_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_WRITE_KHR)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR)
            .dst_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_READ_KHR);
        command_buffer.cmd_pipeline_barrier2(
            &vk::DependencyInfoKHR::builder()
                .memory_barriers(std::slice::from_ref(&barrier))
                .build(),
        );

        // コマンド終了とサブミット
        command_buffer.end_command_buffer();
        let buffers_to_submit = [*command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&buffers_to_submit)
            .build();
        let fence = create_fence(&device);
        device.queue_submit(
            queue_handles.compute.queue,
            &[submit_info],
            Some(fence.clone()),
        );
        device.wait_fences(&[fence], u64::MAX);

        BlasObjects {
            blas,
            blas_buffer,
            vertex_buffer,
            index_buffer,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceParam {
    pub address_index: u64,
    pub address_vertex: u64,
    pub transform: glam::Mat4,
    pub material_index: u32,
    pub padding_1: u32,
    pub padding_2: u64,
}

#[derive(Clone)]
pub struct TlasObjects {
    pub tlas: crate::AccelerationStructureHandle,
    pub tlas_buffer: BufferObjects,
    pub instance_params_buffer: BufferObjects,
    pub materials_buffer: BufferObjects,
}

pub fn create_tlas<Material>(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    compute_command_pool: &crate::CommandPoolHandle,
    transfer_command_pool: &crate::CommandPoolHandle,
    allocator: &crate::AllocatorHandle,
    instancies: &[(BlasObjects, glam::Mat4, u32)],
    materials: &[Material],
) -> TlasObjects {
    // instancesを作成
    let instancies_data = instancies
        .iter()
        .map(
            |(blas, transform, _)| vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: transform.transpose().to_cols_array()[..12]
                        .try_into()
                        .unwrap(),
                },
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
                ),
                instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xFF),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: blas.blas.get_acceleration_structure_device_address(),
                },
            },
        )
        .collect::<Vec<_>>();

    // instanciesのbufferを作成
    let instances_buffer = create_host_buffer_with_data(
        &device,
        &allocator,
        &instancies_data,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
    );

    // geometryを作成
    let instance_data_device_address = vk::DeviceOrHostAddressConstKHR {
        device_address: instances_buffer.device_address,
    };
    let geometry = vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::INSTANCES)
        .geometry(vk::AccelerationStructureGeometryDataKHR {
            instances: *vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                .array_of_pointers(false)
                .data(instance_data_device_address),
        })
        .flags(vk::GeometryFlagsKHR::OPAQUE);

    // build geometry infoを作成
    let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
        .geometries(std::slice::from_ref(&geometry))
        .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
        .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
        .src_acceleration_structure(vk::AccelerationStructureKHR::null());

    // TLASに必要なバッファサイズを取得
    let primitive_count = 1;
    let build_size_info = device.get_acceleration_structure_build_sizes(
        vk::AccelerationStructureBuildTypeKHR::DEVICE,
        &build_geometry_info,
        // &[primitive_count],
        &[primitive_count],
    );

    // bufferの作成
    let tlas_buffer = create_device_local_buffer(
        &device,
        &allocator,
        build_size_info.acceleration_structure_size,
        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    // tlasの作成
    let tlas = device.create_acceleration_structure(
        &vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(*tlas_buffer.buffer)
            .size(build_size_info.acceleration_structure_size)
            .offset(0)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL),
    );

    // scratch bufferの作成
    let scratch_buffer = create_device_local_buffer(
        &device,
        &allocator,
        build_size_info.build_scratch_size,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    // acceleration structureのビルドコマンド実行
    let (tlas, tlas_buffer) = {
        // build用にbuild geometry infoを作成
        let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .geometries(std::slice::from_ref(&geometry))
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .src_acceleration_structure(vk::AccelerationStructureKHR::null())
            .dst_acceleration_structure(*tlas)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: device.get_buffer_device_address(
                    &vk::BufferDeviceAddressInfo::builder()
                        .buffer(*scratch_buffer.buffer)
                        .build(),
                ),
            });
        // build range infoを作成
        let acceleration_structure_build_range_info =
            vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .primitive_count(primitive_count)
                // .primitive_count(0)
                .first_vertex(0)
                .primitive_offset(0)
                .transform_offset(0);

        // コマンドバッファの開始
        let command_buffer = {
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(**compute_command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffers = device
                .allocate_command_buffers(&compute_command_pool, &command_buffer_allocate_info);
            command_buffers.into_iter().next().unwrap()
        };
        begin_onetime_command_buffer(&command_buffer);

        // コマンドのレコード
        command_buffer.cmd_build_acceleration_structures(
            std::slice::from_ref(&build_geometry_info),
            &[std::slice::from_ref(
                &acceleration_structure_build_range_info,
            )],
        );
        let barrier = vk::MemoryBarrier2KHR::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR)
            .src_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_WRITE_KHR)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR)
            .dst_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_READ_KHR);
        command_buffer.cmd_pipeline_barrier2(
            &vk::DependencyInfoKHR::builder()
                .memory_barriers(std::slice::from_ref(&barrier))
                .build(),
        );

        // コマンド終了とサブミット
        command_buffer.end_command_buffer();
        let buffers_to_submit = [*command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&buffers_to_submit)
            .build();
        let fence = create_fence(&device);
        device.queue_submit(
            queue_handles.compute.queue,
            &[submit_info],
            Some(fence.clone()),
        );
        device.wait_fences(&[fence], u64::MAX);

        (tlas, tlas_buffer)
    };

    // instance paramのbufferを作成
    let instance_params = instancies
        .iter()
        .map(|(blas, transform, material)| InstanceParam {
            address_index: blas.index_buffer.device_address,
            address_vertex: blas.vertex_buffer.device_address,
            transform: transform.clone(),
            material_index: *material,
            padding_1: 0,
            padding_2: 0,
        })
        .collect::<Vec<_>>();
    let instance_params_buffer = create_device_local_buffer_with_data(
        &device,
        &queue_handles,
        &transfer_command_pool,
        &allocator,
        &instance_params,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
    );

    // materialのbufferを作成
    let materials_buffer = create_device_local_buffer_with_data(
        &device,
        &queue_handles,
        &transfer_command_pool,
        &allocator,
        &materials,
        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::STORAGE_BUFFER,
    );

    TlasObjects {
        tlas,
        tlas_buffer,
        instance_params_buffer,
        materials_buffer,
    }
}

pub struct HitShaderModules {
    pub closest_hit: crate::ShaderModuleHandle,
    pub any_hit: Option<crate::ShaderModuleHandle>,
    pub intersection: Option<crate::ShaderModuleHandle>,
}

pub fn create_ray_tracing_pipelines(
    device: &crate::DeviceHandle,
    raygen_shader_modules: &[crate::ShaderModuleHandle],
    miss_shader_modules: &[crate::ShaderModuleHandle],
    hit_shader_modules: &[HitShaderModules],
    push_constant_ranges: &[vk::PushConstantRange],
) -> (
    crate::RayTracingPipelineHandle,
    crate::PipelineLayoutHandle,
    crate::DescriptorSetLayoutHandle,
) {
    // descriptor set layoutを作成
    let descriptor_set_layout = {
        let layout_acceleration_structure = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR);

        let layout_storage_image = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR);

        let layout_instance_params = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ANY_HIT_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR);

        let layout_materials = vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ANY_HIT_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR);

        let bindings = [
            layout_acceleration_structure.build(),
            layout_storage_image.build(),
            layout_instance_params.build(),
            layout_materials.build(),
        ];

        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let descriptor_set_layout =
            device.create_descriptor_set_layout(&descriptor_set_layout_create_info);

        descriptor_set_layout
    };

    // pipeline layoutを作成
    let pipeline_layout = {
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(std::slice::from_ref(&descriptor_set_layout))
            .push_constant_ranges(push_constant_ranges);

        let pipeline_layout =
            device.create_pipeline_layout(&descriptor_set_layout, &pipeline_layout_create_info);

        pipeline_layout
    };

    // shader stagesの作成
    let raygen_shader_stages = raygen_shader_modules
        .iter()
        .map(|module| {
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(**module)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                .build()
        })
        .collect::<Vec<_>>();
    let miss_shader_stages = miss_shader_modules
        .iter()
        .map(|module| {
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::MISS_KHR)
                .module(**module)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                .build()
        })
        .collect::<Vec<_>>();
    let hit_shader_stages = hit_shader_modules
        .iter()
        .flat_map(|module| {
            let closest_hit = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                .module(*module.closest_hit)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                .build();
            let any_hit = module.any_hit.as_ref().map(|module| {
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::ANY_HIT_KHR)
                    .module(**module)
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                    .build()
            });
            let intersection = module.intersection.as_ref().map(|module| {
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::INTERSECTION_KHR)
                    .module(**module)
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap())
                    .build()
            });
            let mut stages = vec![closest_hit];
            if let Some(any_hit) = any_hit {
                stages.push(any_hit);
            }
            if let Some(intersection) = intersection {
                stages.push(intersection);
            }
            stages
        })
        .collect::<Vec<_>>();

    let mut shader_stages = vec![];
    shader_stages.extend(raygen_shader_stages);
    shader_stages.extend(miss_shader_stages);
    shader_stages.extend(hit_shader_stages);

    // shader groupsの作成
    let mut index = 0;
    let mut raygen_shader_groups = vec![];
    for _ in raygen_shader_modules {
        raygen_shader_groups.push(
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(index)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
        );
        index += 1;
    }
    let mut miss_shader_groups = vec![];
    for _ in miss_shader_modules {
        miss_shader_groups.push(
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(index)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
        );
        index += 1;
    }
    let mut hit_shader_groups = vec![];
    for hit_shader_module in hit_shader_modules {
        let closest_hit_shader_index = index;
        let any_hit_shader_index = if hit_shader_module.any_hit.is_some() {
            index += 1;
            index
        } else {
            vk::SHADER_UNUSED_KHR
        };
        let intersection_shader_index = if hit_shader_module.intersection.is_some() {
            index += 1;
            index
        } else {
            vk::SHADER_UNUSED_KHR
        };
        hit_shader_groups.push(
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(closest_hit_shader_index)
                .any_hit_shader(any_hit_shader_index)
                .intersection_shader(intersection_shader_index)
                .build(),
        );
        index += 1;
    }

    let mut shader_groups = vec![];
    shader_groups.extend(raygen_shader_groups);
    shader_groups.extend(miss_shader_groups);
    shader_groups.extend(hit_shader_groups);

    // pipelineを作成
    let raytracing_pipeline = {
        let pipeline_create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .stages(&shader_stages)
            .groups(&shader_groups)
            .max_pipeline_ray_recursion_depth(1)
            .layout(*pipeline_layout);

        let raytracing_pipeline = device
            .create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(),
                vk::PipelineCache::null(),
                &pipeline_layout,
                std::slice::from_ref(&pipeline_create_info),
            )
            .into_iter()
            .next()
            .unwrap();

        raytracing_pipeline
    };

    (raytracing_pipeline, pipeline_layout, descriptor_set_layout)
}

pub struct SbtItem {
    pub device_address: u64,
    pub stride: u64,
    pub size: u64,
}

pub struct ShaderBindingTable {
    pub buffer: BufferObjects,
    pub raygen_item: SbtItem,
    pub miss_item: SbtItem,
    pub hit_item: SbtItem,
}

pub fn create_shader_binding_table(
    instance: &crate::InstanceHandle,
    physical_device: vk::PhysicalDevice,
    device: &crate::DeviceHandle,
    allocator: &crate::AllocatorHandle,
    ray_tracing_pipeline: &crate::RayTracingPipelineHandle,
    raygen_shader_group_count: u64,
    miss_shader_group_count: u64,
    hit_shader_group_count: u64,
) -> ShaderBindingTable {
    fn align(value: u64, alignment: u64) -> u64 {
        (value + alignment - 1) & !(alignment - 1)
    }

    let raytracing_pipeline_props = {
        let mut physical_device_raytracing_pipeline_properties =
            vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::builder();
        let mut physical_device_properties = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut physical_device_raytracing_pipeline_properties);
        instance.get_physical_device_properties2(physical_device, &mut physical_device_properties);
        physical_device_raytracing_pipeline_properties
    };

    let handle_size = raytracing_pipeline_props.shader_group_handle_size as u64;
    let handle_alignment = raytracing_pipeline_props.shader_group_base_alignment as u64;

    // handle_sizeのalignmentへの切り上げ
    let handle_size_aligned = align(handle_size, handle_alignment);

    // 各グループで必要なサイズを求める
    let base_align = raytracing_pipeline_props.shader_group_base_alignment as u64;
    let size_raygen = align(handle_size_aligned * raygen_shader_group_count, base_align);
    let size_miss = align(handle_size_aligned * miss_shader_group_count, base_align);
    let size_hit = align(handle_size_aligned * hit_shader_group_count, base_align);

    // shader binding tableの確保
    let buffer_size = size_raygen + size_miss + size_hit;
    let buffer = create_host_buffer(
        device,
        allocator,
        buffer_size,
        vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
    );

    // shader groupのhandlesの取得
    let handle_storage_size = handle_size_aligned
        * (raygen_shader_group_count + miss_shader_group_count + hit_shader_group_count); // shader groups count
    let shader_group_handles = ray_tracing_pipeline.get_ray_tracing_shader_group_handles(
        0,
        (raygen_shader_group_count + miss_shader_group_count + hit_shader_group_count) as u32,
        handle_storage_size as usize,
    );

    // shader entryの書き込み
    let ptr = buffer.allocation.mapped_ptr().unwrap().as_ptr();

    // raygen shader groupの書き込み
    let raygen = shader_group_handles
        [(0 as usize)..((handle_size * raygen_shader_group_count) as usize)]
        .to_vec();
    unsafe { std::ptr::copy_nonoverlapping(raygen.as_ptr(), ptr as *mut u8, size_raygen as usize) };
    let raygen_item = SbtItem {
        device_address: buffer.device_address,
        stride: handle_size_aligned,
        size: handle_size_aligned,
    };

    // miss shader groupの書き込み
    let miss = shader_group_handles[((handle_size * raygen_shader_group_count) as usize)
        ..((handle_size * (raygen_shader_group_count + 1)) as usize)]
        .to_vec();
    unsafe {
        std::ptr::copy_nonoverlapping(
            miss.as_ptr(),
            ptr.add(size_raygen as usize) as *mut u8,
            size_miss as usize,
        )
    };
    let miss_item = SbtItem {
        device_address: buffer.device_address + size_raygen,
        stride: handle_size_aligned,
        size: handle_size_aligned,
    };

    // hit shader groupの書き込み
    let hit = shader_group_handles[((handle_size
        * (raygen_shader_group_count + miss_shader_group_count))
        as usize)
        ..((handle_size
            * (raygen_shader_group_count + miss_shader_group_count + hit_shader_group_count))
            as usize)]
        .to_vec();
    unsafe {
        std::ptr::copy_nonoverlapping(
            hit.as_ptr(),
            ptr.add((size_raygen + size_miss) as usize) as *mut u8,
            size_hit as usize,
        )
    };
    let hit_item = SbtItem {
        device_address: buffer.device_address + size_raygen + size_miss,
        stride: handle_size_aligned,
        size: handle_size_aligned,
    };

    ShaderBindingTable {
        buffer,
        raygen_item,
        miss_item,
        hit_item,
    }
}
