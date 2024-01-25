#[cfg(target_os = "linux")]
use ash::extensions::khr::ExternalMemoryFd;
#[cfg(target_os = "windows")]
use ash::extensions::khr::ExternalMemoryWin32;
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
        #[cfg(target_os = "linux")]
        ExternalMemoryFd::name().to_owned(),
        #[cfg(target_os = "windows")]
        ExternalMemoryWin32::name().to_owned(),
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
            && supported_feature_vulkan_12.buffer_device_address == vk::TRUE
            && supported_feature_vulkan_12.descriptor_indexing == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_partially_bound == vk::TRUE
            && supported_feature_vulkan_12.shader_sampled_image_array_non_uniform_indexing
                == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_sampled_image_update_after_bind
                == vk::TRUE
            && supported_feature_vulkan_12.shader_uniform_buffer_array_non_uniform_indexing
                == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_uniform_buffer_update_after_bind
                == vk::TRUE
            && supported_feature_vulkan_12.shader_storage_image_array_non_uniform_indexing
                == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_storage_image_update_after_bind
                == vk::TRUE
            && supported_feature_vulkan_12.shader_storage_buffer_array_non_uniform_indexing
                == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_storage_buffer_update_after_bind
                == vk::TRUE
            && supported_feature_vulkan_12.descriptor_binding_variable_descriptor_count == vk::TRUE
            && supported_feature_vulkan_12.runtime_descriptor_array == vk::TRUE
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
    /// Graphics Queueのindex
    pub graphics_index: u32,
    /// Transfer Queueのindex
    pub transfer_index: u32,
    /// Compute Queueのindex
    pub compute_index: u32,
    /// Present Queueのindex
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
        .descriptor_indexing(true)
        .descriptor_binding_partially_bound(true)
        .shader_sampled_image_array_non_uniform_indexing(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .shader_uniform_buffer_array_non_uniform_indexing(true)
        .descriptor_binding_uniform_buffer_update_after_bind(true)
        .shader_storage_image_array_non_uniform_indexing(true)
        .descriptor_binding_storage_image_update_after_bind(true)
        .shader_storage_buffer_array_non_uniform_indexing(true)
        .descriptor_binding_storage_buffer_update_after_bind(true)
        .runtime_descriptor_array(true)
        .descriptor_binding_variable_descriptor_count(true)
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
    /// Queueのハンドル
    pub queue: vk::Queue,
    /// QueueFamilyのindex
    pub family_index: u32,
    /// Queueのindex
    pub index: u32,
}

/// 各種QueueのQueueHandleを格納する構造体
#[derive(Debug, Clone)]
pub struct QueueHandles {
    /// Graphics QueueのQueueHandle
    pub graphics: QueueHandle,
    /// Transfer QueueのQueueHandle
    pub transfer: QueueHandle,
    /// Compute QueueのQueueHandle
    pub compute: QueueHandle,
    /// Present QueueのQueueHandle
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
    /// SwapchainHandle
    pub swapchain: crate::SwapchainHandle,
    /// Swapchainのimages
    pub swapchain_images: Vec<vk::Image>,
    /// Swapchainのformat
    pub format: vk::Format,
    /// Swapchainのextent
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
        .find(|surface_format| surface_format.format == vk::Format::B8G8R8A8_UNORM)
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
    let swapchain = device.create_swapchain(&swapchain_create_info);

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
