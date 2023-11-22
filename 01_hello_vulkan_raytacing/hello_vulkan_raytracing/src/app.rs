use anyhow::Result;
use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{
            AccelerationStructure, DeferredHostOperations, RayTracingPipeline, Swapchain,
            Synchronization2, TimelineSemaphore,
        },
    },
    vk, Device, Entry, Instance,
};
use bytemuck;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{collections::HashSet, ffi::CStr};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[repr(C)]
struct Vertex {
    position: [f32; 3],
}

pub struct App {
    width: u32,
    height: u32,
    _entry: Entry,
    instance: Instance,
    debug_utils_loader: DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice,
    physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: Device,
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue,
    present_queue: vk::Queue,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    graphics_command_pool: vk::CommandPool,
    image_transfer_command_buffer: vk::CommandBuffer,
    storage_image: vk::Image,
    storage_image_memory: vk::DeviceMemory,
    storage_image_view: vk::ImageView,
    acceleration_structure_loader: AccelerationStructure,
    blas: vk::AccelerationStructureKHR,
    blas_buffer: vk::Buffer,
    blas_memory: vk::DeviceMemory,
    _blas_acceleration_structure_address: u64,
    tlas: vk::AccelerationStructureKHR,
    tlas_buffer: vk::Buffer,
    tlas_memory: vk::DeviceMemory,
    _tlas_acceleration_structure_address: u64,
    raytracing_pipeline_loader: RayTracingPipeline,
    raytracing_pipeline: vk::Pipeline,
    raytracing_pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    sbt_buffer: vk::Buffer,
    sbt_buffer_memory: vk::DeviceMemory,
    raygen_sbt_device_address: u64,
    raygen_sbt_stride: u64,
    raygen_sbt_size: u64,
    miss_sbt_device_address: u64,
    miss_sbt_stride: u64,
    miss_sbt_size: u64,
    hit_sbt_device_address: u64,
    hit_sbt_stride: u64,
    hit_sbt_size: u64,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    render_command_buffers: Vec<vk::CommandBuffer>,
    in_flight_fences: Vec<vk::Fence>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    current_frame: usize,
    dirty_swapchain: bool,
}
impl App {
    const ENABLE_VALIDATION_LAYERS: bool = true;
    const VALIDATION: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];
    const DEVICE_EXTENSIONS: [&'static CStr; 6] = [
        Swapchain::name(),
        Synchronization2::name(),
        TimelineSemaphore::name(),
        RayTracingPipeline::name(),
        AccelerationStructure::name(),
        DeferredHostOperations::name(),
    ];

    unsafe extern "system" fn vulkan_debug_utils_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _p_user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        let severity = match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
            _ => panic!("[UNKNOWN]"),
        };
        let types = match message_types {
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
            _ => panic!("[UNKNOWN]"),
        };
        let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
        println!("[DEBUG]{}{}{:?}", severity, types, message);

        vk::FALSE
    }

    fn init(event_loop: &EventLoop<()>, window: &Window) -> Result<Self> {
        let width = 800;
        let height = 600;

        let entry = unsafe { Entry::load()? };

        // instanceの作成とdebug utilsの設定
        let (instance, debug_utils_loader, debug_messenger) = {
            let app_name = std::ffi::CString::new("Hello Triangle")?;
            let app_info = vk::ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(vk::make_api_version(1, 0, 0, 0))
                .api_version(vk::API_VERSION_1_3);
            let mut debug_utils_messenger_create_info =
                vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .flags(vk::DebugUtilsMessengerCreateFlagsEXT::empty())
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                    )
                    .pfn_user_callback(Some(Self::vulkan_debug_utils_callback))
                    .build();
            let mut extension_names = vec![DebugUtils::name().as_ptr()];
            for &extension in
                ash_window::enumerate_required_extensions(event_loop.raw_display_handle())?
            {
                let name = unsafe { CStr::from_ptr(extension).as_ptr() };
                extension_names.push(name);
            }
            let raw_layer_names = Self::VALIDATION
                .iter()
                .map(|l| std::ffi::CString::new(*l).unwrap())
                .collect::<Vec<_>>();
            let layer_names = raw_layer_names
                .iter()
                .map(|l| l.as_ptr())
                .collect::<Vec<*const i8>>();
            let instance_create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(&extension_names);
            let instance_create_info = if Self::ENABLE_VALIDATION_LAYERS {
                instance_create_info
                    .push_next(&mut debug_utils_messenger_create_info)
                    .enabled_layer_names(&layer_names)
            } else {
                instance_create_info
            };
            let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

            // setup debug utils
            let debug_utils_loader = DebugUtils::new(&entry, &instance);
            let debug_messenger = if Self::ENABLE_VALIDATION_LAYERS {
                unsafe {
                    debug_utils_loader
                        .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)?
                }
            } else {
                vk::DebugUtilsMessengerEXT::null()
            };

            (instance, debug_utils_loader, debug_messenger)
        };

        // surfaceの作成
        let (surface, surface_loader) = unsafe {
            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )?;
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
            (surface, surface_loader)
        };

        // physical deviceの選択
        let mut graphics_index = None;
        let mut transfer_index = None;
        let mut present_index = None;
        let (physical_device, physical_device_memory_properties) = {
            let physical_devices = unsafe { instance.enumerate_physical_devices()? };
            // GraphicsとTransfer、PresentのサポートされているQueueFamilyのある &&
            // 必要なデバイス拡張機能に対応している &&
            // swapchainに対応したフォーマット / presentationモードが一つ以上ある
            // 必要なdevice featuresに対応していることを確認
            let physical_device = physical_devices.into_iter().find(|physical_device| {
                // QueueFamilyの確認
                let queue_families = unsafe {
                    instance.get_physical_device_queue_family_properties(*physical_device)
                };
                for (i, queue_family) in queue_families.iter().enumerate() {
                    if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        graphics_index = Some(i);
                    }
                    if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                        transfer_index = Some(i);
                    }
                    let present_support = unsafe {
                        surface_loader
                            .get_physical_device_surface_support(
                                *physical_device,
                                i as u32,
                                surface,
                            )
                            .unwrap()
                    };
                    if present_support {
                        present_index = Some(i);
                    }
                    if graphics_index.is_some()
                        && transfer_index.is_some()
                        && present_index.is_some()
                    {
                        break;
                    }
                }
                let is_queue_family_supported =
                    graphics_index.is_some() && transfer_index.is_some();

                // デバイス拡張の確認
                let device_extensions = unsafe {
                    instance
                        .enumerate_device_extension_properties(*physical_device)
                        .unwrap()
                };
                let mut device_extensions_name = vec![];
                for device_extension in device_extensions {
                    let name = unsafe {
                        std::ffi::CStr::from_ptr(device_extension.extension_name.as_ptr())
                            .to_owned()
                    };
                    device_extensions_name.push(name);
                }
                let mut required_extensions = HashSet::new();
                for &extension in Self::DEVICE_EXTENSIONS.iter() {
                    required_extensions.insert(extension.to_owned());
                }
                for extension_name in device_extensions_name {
                    required_extensions.remove(&extension_name);
                }
                let is_device_extension_supported = required_extensions.is_empty();

                // swapchainのサポート確認
                let surface_formats = unsafe {
                    surface_loader
                        .get_physical_device_surface_formats(*physical_device, surface)
                        .unwrap()
                };
                let surface_present_modes = unsafe {
                    surface_loader
                        .get_physical_device_surface_present_modes(*physical_device, surface)
                        .unwrap()
                };
                let is_swapchain_supported =
                    !surface_formats.is_empty() && !surface_present_modes.is_empty();

                // featureのサポート確認
                let mut supported_feature_vulkan_12 =
                    vk::PhysicalDeviceVulkan12Features::builder().build();
                let mut supported_feature_vulkan_13 =
                    vk::PhysicalDeviceVulkan13Features::builder().build();
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
                unsafe {
                    instance.get_physical_device_features2(*physical_device, &mut supported_feature)
                };
                let is_supported_device_features = supported_feature_vulkan_12.timeline_semaphore
                    == vk::TRUE
                    && supported_feature_vulkan_13.synchronization2 == vk::TRUE;

                is_queue_family_supported
                    && is_swapchain_supported
                    && is_device_extension_supported
                    && is_supported_device_features
            });
            let physical_device =
                physical_device.ok_or(anyhow::anyhow!("No suitable physical device"))?;
            let physical_device_memory_properties =
                unsafe { instance.get_physical_device_memory_properties(physical_device) };

            (physical_device, physical_device_memory_properties)
        };

        // デバイスとキューの作成
        let (device, graphics_queue, transfer_queue, present_queue) = {
            // queue create info
            let mut unique_queue_families = HashSet::new();
            unique_queue_families.insert(graphics_index.unwrap());
            unique_queue_families.insert(transfer_index.unwrap());
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
            let physical_device_features = vk::PhysicalDeviceFeatures::builder().build();
            let mut physical_device_vulkan_12_features =
                vk::PhysicalDeviceVulkan12Features::builder()
                    .timeline_semaphore(true)
                    .buffer_device_address(true)
                    .build();
            let mut physical_device_vulkan_13_features =
                vk::PhysicalDeviceVulkan13Features::builder()
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
            let enable_extension_names = Self::DEVICE_EXTENSIONS
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
            let device =
                unsafe { instance.create_device(physical_device, &device_create_info, None)? };

            // get device queue
            let graphics_queue =
                unsafe { device.get_device_queue(graphics_index.unwrap() as u32, 0) };
            let transfer_queue =
                unsafe { device.get_device_queue(transfer_index.unwrap() as u32, 0) };
            let present_queue =
                unsafe { device.get_device_queue(present_index.unwrap() as u32, 0) };

            (device, graphics_queue, transfer_queue, present_queue)
        };

        // swapchainの作成
        let (swapchain, swapchain_loader, swapchain_images, swapchain_format, swapchain_extent) = {
            let surface_capabilities = unsafe {
                surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
            };
            let surface_formats = unsafe {
                surface_loader.get_physical_device_surface_formats(physical_device, surface)?
            };
            let surface_present_modes = unsafe {
                surface_loader
                    .get_physical_device_surface_present_modes(physical_device, surface)?
            };

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
                let window_size = window.inner_size();
                vk::Extent2D {
                    width: window_size.width.clamp(
                        surface_capabilities.min_image_extent.width,
                        surface_capabilities.max_image_extent.width,
                    ),
                    height: window_size.height.clamp(
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
                .surface(surface)
                .min_image_count(image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_extent)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(*surface_present_mode)
                .image_array_layers(1)
                .clipped(true);
            let swapchain_loader = Swapchain::new(&instance, &device);
            let swapchain =
                unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

            // swapchainのimageの取得
            let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

            (
                swapchain,
                swapchain_loader,
                swapchain_images,
                surface_format.format,
                surface_extent,
            )
        };

        // command poolを作成
        let command_pool = {
            let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(graphics_index.unwrap() as u32);
            unsafe { device.create_command_pool(&command_pool_create_info, None)? }
        };

        // command bufferの作成
        let image_transfer_command_buffer = {
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffers =
                unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }?;
            command_buffers[0]
        };

        // 描画先のstorage imageの作成
        let (storage_image, storage_image_memory, storage_image_view) = {
            // imageの生成
            let image_create_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let image = unsafe { device.create_image(&image_create_info, None)? };

            // imageのメモリ確保
            let image_memory_requirement = unsafe { device.get_image_memory_requirements(image) };
            let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
            let memory_type_index = physical_device_memory_properties
                .memory_types
                .iter()
                .enumerate()
                .position(|(i, memory_type)| {
                    let is_required_memory_type =
                        (image_memory_requirement.memory_type_bits & (1 << i)) > 0
                            && memory_type
                                .property_flags
                                .contains(required_memory_properties);
                    is_required_memory_type
                })
                .ok_or(anyhow::anyhow!("No suitable memory type"))?
                as u32;
            let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(image_memory_requirement.size)
                .memory_type_index(memory_type_index);
            let image_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

            // imageとメモリのバインド
            unsafe { device.bind_image_memory(image, image_memory, 0)? };

            // image_viewの作成
            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
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
                .image(image);
            let image_view = unsafe { device.create_image_view(&image_view_create_info, None)? };

            // 画像のレイアウトを UNDEFINED -> GENERAL に変更する
            {
                // コマンドバッファの開始
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                unsafe {
                    device.begin_command_buffer(
                        image_transfer_command_buffer,
                        &command_buffer_begin_info,
                    )
                }?;

                // 画像レイアウト変更のコマンドのレコード
                let image_barriers = [vk::ImageMemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2KHR::TOP_OF_PIPE)
                    .src_access_mask(vk::AccessFlags2KHR::empty())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .dst_stage_mask(vk::PipelineStageFlags2KHR::BOTTOM_OF_PIPE)
                    .dst_access_mask(vk::AccessFlags2KHR::empty())
                    .new_layout(vk::ImageLayout::GENERAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    )
                    .image(image)
                    .build()];
                unsafe {
                    device.cmd_pipeline_barrier2(
                        image_transfer_command_buffer,
                        &vk::DependencyInfoKHR::builder()
                            .image_memory_barriers(&image_barriers)
                            .build(),
                    );
                }

                // コマンドバッファの終了
                unsafe { device.end_command_buffer(image_transfer_command_buffer) }?;

                // コマンドバッファのサブミット
                let buffers_to_submit = [image_transfer_command_buffer];
                let submit_info = vk::SubmitInfo::builder()
                    .command_buffers(&buffers_to_submit)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])
                    .wait_semaphores(&[])
                    .build();
                unsafe {
                    device.queue_submit(transfer_queue, &[submit_info], vk::Fence::null())?;
                }

                // コマンド完了まで待機
                unsafe { device.device_wait_idle()? };
            }

            (image, image_memory, image_view)
        };

        // vertex bufferを作成
        let (vertex_buffer, vertex_buffer_memory, vertex_buffer_address) = {
            // 頂点データ
            let vertices = [
                Vertex {
                    position: [0.0, -0.5, 0.0],
                },
                Vertex {
                    position: [-0.5, 0.5, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5, 0.0],
                },
            ];
            let buffer_size = std::mem::size_of_val(&vertices) as u64;

            // vertex bufferを作成
            let (vertex_buffer, vertex_buffer_memory) = {
                // bufferの作成
                let buffer_create_info = vk::BufferCreateInfo::builder().size(buffer_size).usage(
                    vk::BufferUsageFlags::VERTEX_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties =
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // 頂点データのコピー
            let data = unsafe {
                device.map_memory(
                    vertex_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )?
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    vertices.as_ptr() as *const u8,
                    data as *mut u8,
                    buffer_size as usize,
                )
            };

            // device addressの取得
            let vertex_buffer_address = unsafe {
                device.get_buffer_device_address(
                    &vk::BufferDeviceAddressInfo::builder().buffer(vertex_buffer),
                )
            };

            (vertex_buffer, vertex_buffer_memory, vertex_buffer_address)
        };

        // index bufferを作成
        let (index_buffer, index_buffer_memory, index_buffer_address) = {
            // 頂点データ
            let vertices: [u32; 3] = [0, 1, 2];
            let buffer_size = std::mem::size_of_val(&vertices) as u64;

            // staging bufferを作成
            let (index_buffer, index_buffer_memory) = {
                // bufferの作成
                let buffer_create_info = vk::BufferCreateInfo::builder().size(buffer_size).usage(
                    vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties =
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // 頂点データのコピー
            let data = unsafe {
                device.map_memory(
                    index_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )?
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    vertices.as_ptr() as *const u8,
                    data as *mut u8,
                    buffer_size as usize,
                )
            };

            // device addressの取得
            let index_buffer_address = unsafe {
                device.get_buffer_device_address(
                    &vk::BufferDeviceAddressInfo::builder().buffer(index_buffer),
                )
            };

            (index_buffer, index_buffer_memory, index_buffer_address)
        };

        // acceleration structure loaderの作成
        let acceleration_structure_loader = AccelerationStructure::new(&instance, &device);

        // Blasの作成
        let (blas, blas_buffer, blas_memory, blas_acceleration_structure_address) = {
            // geometryを作成
            let geometry_triangle_date =
                vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                    .vertex_format(vk::Format::R32G32B32_SFLOAT)
                    .vertex_data(vk::DeviceOrHostAddressConstKHR {
                        device_address: vertex_buffer_address,
                    })
                    .vertex_stride(std::mem::size_of::<Vertex>() as u64)
                    .index_type(vk::IndexType::UINT32)
                    .index_data(vk::DeviceOrHostAddressConstKHR {
                        device_address: index_buffer_address,
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
            let primitive_count = 1;
            let build_size_info = unsafe {
                acceleration_structure_loader.get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_geometry_info,
                    &[primitive_count],
                )
            };

            // バッファを確保
            let (buffer, buffer_memory) = {
                let buffer_create_info = vk::BufferCreateInfo::builder()
                    .size(build_size_info.acceleration_structure_size)
                    .usage(
                        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                    );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // blasの作成
            let blas = unsafe {
                acceleration_structure_loader.create_acceleration_structure(
                    &vk::AccelerationStructureCreateInfoKHR::builder()
                        .buffer(buffer)
                        .size(build_size_info.acceleration_structure_size)
                        .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL),
                    None,
                )?
            };

            // scratch bufferの作成
            let (scratch_buffer, scratch_buffer_memory) = {
                let buffer_create_info = vk::BufferCreateInfo::builder()
                    .size(build_size_info.build_scratch_size)
                    .usage(
                        vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                    );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // acceleration structureのビルドコマンド実行
            {
                // build用にbuild geometry infoを作成
                let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                    .geometries(std::slice::from_ref(&geometry))
                    .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                    .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                    .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                    .src_acceleration_structure(vk::AccelerationStructureKHR::null())
                    .dst_acceleration_structure(blas)
                    .scratch_data(vk::DeviceOrHostAddressKHR {
                        device_address: unsafe {
                            device.get_buffer_device_address(
                                &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
                            )
                        },
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
                        .command_pool(command_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1);
                    let command_buffers =
                        unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }?;
                    command_buffers[0]
                };
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info) }?;

                // コマンドのレコード
                unsafe {
                    // acceleration structureのビルド
                    acceleration_structure_loader.cmd_build_acceleration_structures(
                        command_buffer,
                        std::slice::from_ref(&build_geometry_info),
                        &[std::slice::from_ref(
                            &acceleration_structure_build_range_info,
                        )],
                    );

                    // メモリバリア
                    let barrier = vk::MemoryBarrier2KHR::builder()
                        .src_stage_mask(
                            vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR,
                        )
                        .src_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_WRITE_KHR)
                        .dst_stage_mask(
                            vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR,
                        )
                        .dst_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_READ_KHR);
                    device.cmd_pipeline_barrier2(
                        command_buffer,
                        &vk::DependencyInfoKHR::builder()
                            .memory_barriers(std::slice::from_ref(&barrier))
                            .build(),
                    );
                };

                // コマンド終了とサブミット
                unsafe { device.end_command_buffer(command_buffer) }?;
                let buffers_to_submit = [command_buffer];
                let submit_info = vk::SubmitInfo::builder()
                    .command_buffers(&buffers_to_submit)
                    .build();
                unsafe {
                    device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
                    device.queue_wait_idle(graphics_queue)?;
                    device.free_command_buffers(command_pool, &buffers_to_submit);
                }
            }

            // scratch bufferの解放
            unsafe {
                device.destroy_buffer(scratch_buffer, None);
                device.free_memory(scratch_buffer_memory, None);
            }

            // acceleration_structure_addressの取得
            let acceleration_structure_address = unsafe {
                acceleration_structure_loader.get_acceleration_structure_device_address(
                    &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                        .acceleration_structure(blas),
                )
            };

            (blas, buffer, buffer_memory, acceleration_structure_address)
        };

        // vertex bufferとindex bufferの解放
        unsafe {
            device.destroy_buffer(vertex_buffer, None);
            device.free_memory(vertex_buffer_memory, None);
            device.destroy_buffer(index_buffer, None);
            device.free_memory(index_buffer_memory, None);
        }

        // tlasの作成
        let (tlas, tlas_buffer, tlas_memory, tlas_acceleration_structure_address) = {
            // instanceを作成
            let transform_matrix = vk::TransformMatrixKHR {
                matrix: glam::Mat4::IDENTITY.transpose().to_cols_array()[..12]
                    .try_into()
                    .unwrap(),
            };
            let instance = vk::AccelerationStructureInstanceKHR {
                transform: transform_matrix,
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
                ),
                instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xFF),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: blas_acceleration_structure_address,
                },
            };
            let instancies = [instance];

            // instancesのbuffer size
            let instances_buffer_size = std::mem::size_of_val(&instancies) as u64;

            // instancesのbufferの作成
            let (instances_buffer, instances_buffer_memory) = {
                let buffer_create_info = vk::BufferCreateInfo::builder().size(instances_buffer_size).usage(
                    vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties =
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // データのコピー
            let data = unsafe {
                device.map_memory(
                    instances_buffer_memory,
                    0,
                    instances_buffer_size,
                    vk::MemoryMapFlags::empty(),
                )?
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    instancies.as_ptr() as *const u8,
                    data as *mut u8,
                    instances_buffer_size as usize,
                )
            };

            // geometryを作成
            let instance_data_device_address = vk::DeviceOrHostAddressConstKHR {
                device_address: unsafe {
                    device.get_buffer_device_address(
                        &vk::BufferDeviceAddressInfo::builder().buffer(instances_buffer),
                    )
                },
            };
            let geometry = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                        .array_of_pointers(false)
                        .data(instance_data_device_address)
                        .build(),
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
            let build_size_info = unsafe {
                acceleration_structure_loader.get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &build_geometry_info,
                    &[primitive_count],
                )
            };

            // バッファを確保
            let (buffer, buffer_memory) = {
                let buffer_create_info = vk::BufferCreateInfo::builder()
                    .size(build_size_info.acceleration_structure_size)
                    .usage(
                        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                    );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // tlasの作成
            let tlas = unsafe {
                acceleration_structure_loader.create_acceleration_structure(
                    &vk::AccelerationStructureCreateInfoKHR::builder()
                        .buffer(buffer)
                        .size(build_size_info.acceleration_structure_size)
                        .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL),
                    None,
                )?
            };

            // scratch bufferの作成
            let (scratch_buffer, scratch_buffer_memory) = {
                let buffer_create_info = vk::BufferCreateInfo::builder()
                    .size(build_size_info.build_scratch_size)
                    .usage(
                        vk::BufferUsageFlags::STORAGE_BUFFER
                            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                    );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                (buffer, buffer_memory)
            };

            // acceleration structureのビルドコマンド実行
            {
                // build用にbuild geometry infoを作成
                let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                    .geometries(std::slice::from_ref(&geometry))
                    .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
                    .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                    .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                    .src_acceleration_structure(vk::AccelerationStructureKHR::null())
                    .dst_acceleration_structure(tlas)
                    .scratch_data(vk::DeviceOrHostAddressKHR {
                        device_address: unsafe {
                            device.get_buffer_device_address(
                                &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
                            )
                        },
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
                        .command_pool(command_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1);
                    let command_buffers =
                        unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }?;
                    command_buffers[0]
                };
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info) }?;

                // コマンドのレコード
                unsafe {
                    // acceleration structureのビルド
                    acceleration_structure_loader.cmd_build_acceleration_structures(
                        command_buffer,
                        std::slice::from_ref(&build_geometry_info),
                        &[std::slice::from_ref(
                            &acceleration_structure_build_range_info,
                        )],
                    );

                    // メモリバリア
                    let barrier = vk::MemoryBarrier2KHR::builder()
                        .src_stage_mask(
                            vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR,
                        )
                        .src_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_WRITE_KHR)
                        .dst_stage_mask(
                            vk::PipelineStageFlags2KHR::ACCELERATION_STRUCTURE_BUILD_KHR,
                        )
                        .dst_access_mask(vk::AccessFlags2KHR::ACCELERATION_STRUCTURE_READ_KHR);
                    device.cmd_pipeline_barrier2(
                        command_buffer,
                        &vk::DependencyInfoKHR::builder()
                            .memory_barriers(std::slice::from_ref(&barrier))
                            .build(),
                    );
                };

                // コマンド終了とサブミット
                unsafe { device.end_command_buffer(command_buffer) }?;
                let buffers_to_submit = [command_buffer];
                let submit_info = vk::SubmitInfo::builder()
                    .command_buffers(&buffers_to_submit)
                    .build();
                unsafe {
                    device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
                    device.queue_wait_idle(graphics_queue)?;
                    device.free_command_buffers(command_pool, &buffers_to_submit);
                }
            }

            // scratch bufferの解放
            unsafe {
                device.destroy_buffer(scratch_buffer, None);
                device.free_memory(scratch_buffer_memory, None);
            }

            // instances bufferの解放
            unsafe {
                device.destroy_buffer(instances_buffer, None);
                device.free_memory(instances_buffer_memory, None);
            }

            // acceleration_structure_addressの取得
            let acceleration_structure_address = unsafe {
                acceleration_structure_loader.get_acceleration_structure_device_address(
                    &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                        .acceleration_structure(blas),
                )
            };

            (tlas, buffer, buffer_memory, acceleration_structure_address)
        };

        // raytracing pipeline loader
        let raytracing_pipeline_loader = RayTracingPipeline::new(&instance, &device);

        // raytracing pipelineを作成
        let (raytracing_pipeline, raytracing_pipeline_layout, descriptor_set_layout) = {
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

                let bindings = [
                    layout_acceleration_structure.build(),
                    layout_storage_image.build(),
                ];

                let descriptor_set_layout_create_info =
                    vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

                let descriptor_set_layout = unsafe {
                    device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)?
                };

                descriptor_set_layout
            };

            // pipeline layoutを作成
            let pipeline_layout = {
                let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(std::slice::from_ref(&descriptor_set_layout));

                let pipeline_layout =
                    unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

                pipeline_layout
            };

            // shader modulesの作成
            let raygen_shader_module = {
                let raygen_shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(
                    bytemuck::cast_slice(include_bytes!("./shaders/spv/raygen.rgen.spv")),
                );

                unsafe { device.create_shader_module(&raygen_shader_module_create_info, None)? }
            };
            let miss_shader_module = {
                let miss_shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(
                    bytemuck::cast_slice(include_bytes!("./shaders/spv/miss.rmiss.spv")),
                );

                unsafe { device.create_shader_module(&miss_shader_module_create_info, None)? }
            };
            let hit_shader_module = {
                let closest_hit_shader_module_create_info = vk::ShaderModuleCreateInfo::builder()
                    .code(bytemuck::cast_slice(include_bytes!(
                        "./shaders/spv/closesthit.rchit.spv"
                    )));

                unsafe {
                    device.create_shader_module(&closest_hit_shader_module_create_info, None)?
                }
            };

            // shader stagesの作成
            let raygen_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(raygen_shader_module)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap());
            let miss_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::MISS_KHR)
                .module(miss_shader_module)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap());
            let hit_shader_stage = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                .module(hit_shader_module)
                .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap());

            let shader_stages = [
                raygen_shader_stage.build(),
                miss_shader_stage.build(),
                hit_shader_stage.build(),
            ];

            // shader stagesのindex
            let raygen_shader_stage_index = 0;
            let miss_shader_stage_index = 1;
            let hit_shader_stage_index = 2;

            // shader groupsの作成
            let raygen_shader_group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(raygen_shader_stage_index)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR);
            let miss_shader_group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(miss_shader_stage_index)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR);
            let closest_hit_shader_group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(hit_shader_stage_index)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR);

            let shader_groups = [
                raygen_shader_group.build(),
                miss_shader_group.build(),
                closest_hit_shader_group.build(),
            ];

            // pipelineを作成
            let raytracing_pipeline = {
                let pipeline_create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
                    .stages(&shader_stages)
                    .groups(&shader_groups)
                    .max_pipeline_ray_recursion_depth(1)
                    .layout(pipeline_layout);

                let raytracing_pipeline = unsafe {
                    raytracing_pipeline_loader.create_ray_tracing_pipelines(
                        vk::DeferredOperationKHR::null(),
                        vk::PipelineCache::null(),
                        std::slice::from_ref(&pipeline_create_info),
                        None,
                    )
                }?[0];

                // shader moduleの解放
                unsafe {
                    device.destroy_shader_module(raygen_shader_module, None);
                    device.destroy_shader_module(miss_shader_module, None);
                    device.destroy_shader_module(hit_shader_module, None);
                }

                raytracing_pipeline
            };

            (raytracing_pipeline, pipeline_layout, descriptor_set_layout)
        };

        // shader binding tableを作成
        let (
            sbt_buffer,
            sbt_buffer_memory,
            raygen_sbt_device_address,
            raygen_sbt_stride,
            raygen_sbt_size,
            miss_sbt_device_address,
            miss_sbt_stride,
            miss_sbt_size,
            hit_sbt_device_address,
            hit_sbt_stride,
            hit_sbt_size,
        ) = {
            let raytracing_pipeline_props = {
                let mut physical_device_raytracing_pipeline_properties =
                    vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::builder();
                let mut physical_device_properties = vk::PhysicalDeviceProperties2::builder()
                    .push_next(&mut physical_device_raytracing_pipeline_properties);
                unsafe {
                    instance.get_physical_device_properties2(
                        physical_device,
                        &mut physical_device_properties,
                    );
                };
                physical_device_raytracing_pipeline_properties
            };

            let handle_size = raytracing_pipeline_props.shader_group_handle_size as u64;
            let handle_alignment = raytracing_pipeline_props.shader_group_base_alignment as u64;

            // handle_sizeのalignmentへの切り上げ
            let handle_size_aligned =
                (handle_size + handle_alignment - 1) & !(handle_alignment - 1);

            // 各groupの中のshaderの個数
            let raygen_shader_group_count = 1;
            let miss_shader_group_count = 1;
            let hit_shader_group_count = 1;

            // 各グループで必要なサイズを求める
            let base_align = raytracing_pipeline_props.shader_group_base_alignment as u64;
            let size_raygen = (handle_size_aligned * raygen_shader_group_count + base_align - 1)
                & !(base_align - 1);
            let size_miss = (handle_size_aligned * miss_shader_group_count + base_align - 1)
                & !(base_align - 1);
            let size_hit =
                (handle_size_aligned * hit_shader_group_count + base_align - 1) & !(base_align - 1);

            // shader binding tableの確保
            let buffer_size = size_raygen + size_miss + size_hit;
            let (buffer, buffer_memory, buffer_device_address) = {
                let buffer_create_info = vk::BufferCreateInfo::builder().size(buffer_size).usage(
                    vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                );
                let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

                // bufferのメモリ確保
                let buffer_memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(buffer) };
                let required_memory_properties =
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                let memory_type_index = physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (buffer_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))?
                    as u32;
                let mut memory_allocate_flags_info = vk::MemoryAllocateFlagsInfo::builder()
                    .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS_KHR);
                let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(buffer_memory_requirement.size)
                    .memory_type_index(memory_type_index)
                    .push_next(&mut memory_allocate_flags_info);
                let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None)? };

                // bufferとメモリのバインド
                unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

                // buffer device addressの取得
                let buffer_device_address = unsafe {
                    device.get_buffer_device_address(
                        &vk::BufferDeviceAddressInfo::builder().buffer(buffer),
                    )
                };

                (buffer, buffer_memory, buffer_device_address)
            };

            // shader groupのhandlesの取得
            let handle_storage_size = handle_size_aligned * 3; // shader groups count
            let shader_group_handles = unsafe {
                raytracing_pipeline_loader.get_ray_tracing_shader_group_handles(
                    raytracing_pipeline,
                    0,
                    3,
                    handle_storage_size as usize,
                )
            }?;

            // shader entryの書き込み
            let data = unsafe {
                device.map_memory(buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
            };

            // raygen shader groupの書き込み
            // raygenはshader groupsの0番目
            let raygen = shader_group_handles[0..handle_size as usize].to_vec();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    raygen.as_ptr(),
                    data as *mut u8,
                    size_raygen as usize,
                )
            };
            let raygen_sbt_device_address = buffer_device_address + 0;
            let raygen_sbt_stride = handle_size_aligned;
            let raygen_sbt_size = raygen_sbt_stride;

            // miss shader groupの書き込み
            // missはshader groupsの1番目
            let miss =
                shader_group_handles[handle_size as usize..(handle_size * 2) as usize].to_vec();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    miss.as_ptr(),
                    data.add(size_raygen as usize) as *mut u8,
                    size_miss as usize,
                )
            };
            let miss_sbt_device_address = buffer_device_address + size_raygen;
            let miss_sbt_stride = handle_size_aligned;
            let miss_sbt_size = miss_sbt_stride;

            // hit shader groupの書き込み
            // hitはshader groupsの2番目
            let hit = shader_group_handles[(handle_size * 2) as usize..(handle_size * 3) as usize]
                .to_vec();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    hit.as_ptr(),
                    data.add((size_raygen + size_miss) as usize) as *mut u8,
                    size_hit as usize,
                )
            };
            let hit_sbt_device_address = buffer_device_address + size_raygen + size_miss;
            let hit_sbt_stride = handle_size_aligned;
            let hit_sbt_size = hit_sbt_stride;

            (
                buffer,
                buffer_memory,
                raygen_sbt_device_address,
                raygen_sbt_stride,
                raygen_sbt_size,
                miss_sbt_device_address,
                miss_sbt_stride,
                miss_sbt_size,
                hit_sbt_device_address,
                hit_sbt_stride,
                hit_sbt_size,
            )
        };

        // descriptor poolの作成
        let descriptor_pool = {
            let descriptor_pool_size_acceleration_structure = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1);
            let descriptor_pool_size_storage_image = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1);
            let descriptor_pool_sizes = [
                descriptor_pool_size_acceleration_structure.build(),
                descriptor_pool_size_storage_image.build(),
            ];

            let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&descriptor_pool_sizes)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

            unsafe { device.create_descriptor_pool(&descriptor_pool_create_info, None)? }
        };

        // descriptor setの作成
        let descriptor_set = {
            // descriptor setのアロケート
            let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&descriptor_set_layout));
            let descriptor_set =
                unsafe { device.allocate_descriptor_sets(&descriptor_set_allocate_info) }?[0];

            // acceleration structureの書き込み
            let mut descriptor_acceleration_structure_info =
                vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                    .acceleration_structures(std::slice::from_ref(&tlas));
            let mut acceleration_structure_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .push_next(&mut descriptor_acceleration_structure_info);
            acceleration_structure_write.descriptor_count = 1;

            // storage imageの書き込み
            let storage_image_info = vk::DescriptorImageInfo::builder()
                .image_view(storage_image_view)
                .image_layout(vk::ImageLayout::GENERAL);
            let storage_image_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&storage_image_info));

            // descriptor setの更新
            let descriptor_writes = [
                acceleration_structure_write.build(),
                storage_image_write.build(),
            ];
            unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };

            descriptor_set
        };

        // command bufferを作成
        let render_command_buffers = {
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(swapchain_images.len() as u32);
            let command_buffers =
                unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }?;
            command_buffers
        };

        // fenceの作成
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let mut in_flight_fences = vec![];
        for _ in 0..swapchain_images.len() {
            let fence = unsafe { device.create_fence(&fence_create_info, None)? };
            in_flight_fences.push(fence);
        }

        // semaphoreの作成
        let mut image_available_semaphores = vec![];
        for _ in 0..swapchain_images.len() {
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let timeline_semaphore =
                unsafe { device.create_semaphore(&semaphore_create_info, None)? };
            image_available_semaphores.push(timeline_semaphore);
        }
        let mut render_finished_semaphores = vec![];
        for _ in 0..swapchain_images.len() {
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let timeline_semaphore =
                unsafe { device.create_semaphore(&semaphore_create_info, None)? };
            render_finished_semaphores.push(timeline_semaphore);
        }

        Ok(Self {
            width,
            height,
            _entry: entry,
            instance,
            debug_utils_loader,
            debug_messenger,
            physical_device,
            physical_device_memory_properties,
            device,
            graphics_queue,
            transfer_queue,
            present_queue,
            surface,
            surface_loader,
            swapchain_loader,
            swapchain,
            swapchain_images,
            swapchain_format,
            swapchain_extent,
            graphics_command_pool: command_pool,
            image_transfer_command_buffer,
            storage_image,
            storage_image_memory,
            storage_image_view,
            acceleration_structure_loader,
            blas,
            blas_buffer,
            blas_memory,
            _blas_acceleration_structure_address: blas_acceleration_structure_address,
            tlas,
            tlas_buffer,
            tlas_memory,
            _tlas_acceleration_structure_address: tlas_acceleration_structure_address,
            raytracing_pipeline_loader,
            raytracing_pipeline,
            raytracing_pipeline_layout,
            descriptor_set_layout,
            sbt_buffer,
            sbt_buffer_memory,
            raygen_sbt_device_address,
            raygen_sbt_stride,
            raygen_sbt_size,
            miss_sbt_device_address,
            miss_sbt_stride,
            miss_sbt_size,
            hit_sbt_device_address,
            hit_sbt_stride,
            hit_sbt_size,
            descriptor_pool,
            descriptor_set,
            render_command_buffers,
            in_flight_fences,
            image_available_semaphores,
            render_finished_semaphores,
            current_frame: 0,
            dirty_swapchain: false,
        })
    }

    fn recreate_swapchain(&mut self, width: u32, height: u32) -> Result<()> {
        // deviceのidle待機
        unsafe { self.device.device_wait_idle()? };

        // swapchainとstorage image, descriptor_set, sync objectsのcleanup
        unsafe {
            for &fence in self.in_flight_fences.iter() {
                self.device.destroy_fence(fence, None);
            }
            for &semaphore in self.image_available_semaphores.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in self.render_finished_semaphores.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }
            self.device
                .free_descriptor_sets(self.descriptor_pool, &[self.descriptor_set])?;
            self.device
                .destroy_image_view(self.storage_image_view, None);
            self.device.destroy_image(self.storage_image, None);
            self.device.free_memory(self.storage_image_memory, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }

        // 新しいサイズ
        self.width = width;
        self.height = height;

        // swapchainの作成
        let (swapchain, swapchain_images, swapchain_format, swapchain_extent) = {
            let surface_capabilities = unsafe {
                self.surface_loader
                    .get_physical_device_surface_capabilities(self.physical_device, self.surface)?
            };
            let surface_formats = unsafe {
                self.surface_loader
                    .get_physical_device_surface_formats(self.physical_device, self.surface)?
            };
            let surface_present_modes = unsafe {
                self.surface_loader
                    .get_physical_device_surface_present_modes(self.physical_device, self.surface)?
            };

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
                    width: self.width.clamp(
                        surface_capabilities.min_image_extent.width,
                        surface_capabilities.max_image_extent.width,
                    ),
                    height: self.height.clamp(
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
                .surface(self.surface)
                .min_image_count(image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_extent)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(*surface_present_mode)
                .image_array_layers(1)
                .clipped(true);
            let swapchain = unsafe {
                self.swapchain_loader
                    .create_swapchain(&swapchain_create_info, None)?
            };

            // swapchainのimageの取得
            let swapchain_images =
                unsafe { self.swapchain_loader.get_swapchain_images(swapchain)? };

            (
                swapchain,
                swapchain_images,
                surface_format.format,
                surface_extent,
            )
        };
        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_format = swapchain_format;
        self.swapchain_extent = swapchain_extent;

        // 描画先のstorage imageの作成
        let (storage_image, storage_image_memory, storage_image_view) = {
            // imageの生成
            let image_create_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let image = unsafe { self.device.create_image(&image_create_info, None)? };

            // imageのメモリ確保
            let image_memory_requirement =
                unsafe { self.device.get_image_memory_requirements(image) };
            let required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
            let memory_type_index =
                self.physical_device_memory_properties
                    .memory_types
                    .iter()
                    .enumerate()
                    .position(|(i, memory_type)| {
                        let is_required_memory_type =
                            (image_memory_requirement.memory_type_bits & (1 << i)) > 0
                                && memory_type
                                    .property_flags
                                    .contains(required_memory_properties);
                        is_required_memory_type
                    })
                    .ok_or(anyhow::anyhow!("No suitable memory type"))? as u32;
            let memory_allocate_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(image_memory_requirement.size)
                .memory_type_index(memory_type_index);
            let image_memory = unsafe { self.device.allocate_memory(&memory_allocate_info, None)? };

            // imageとメモリのバインド
            unsafe { self.device.bind_image_memory(image, image_memory, 0)? };

            // image_viewの作成
            let image_view_create_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
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
                .image(image);
            let image_view = unsafe {
                self.device
                    .create_image_view(&image_view_create_info, None)?
            };

            // 画像のレイアウトを UNDEFINED -> GENERAL に変更する
            {
                // コマンドバッファの開始
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                unsafe {
                    self.device.begin_command_buffer(
                        self.image_transfer_command_buffer,
                        &command_buffer_begin_info,
                    )
                }?;

                // 画像レイアウト変更のコマンドのレコード
                let image_barriers = [vk::ImageMemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2KHR::TOP_OF_PIPE)
                    .src_access_mask(vk::AccessFlags2KHR::empty())
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .dst_stage_mask(vk::PipelineStageFlags2KHR::BOTTOM_OF_PIPE)
                    .dst_access_mask(vk::AccessFlags2KHR::empty())
                    .new_layout(vk::ImageLayout::GENERAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    )
                    .image(image)
                    .build()];
                unsafe {
                    self.device.cmd_pipeline_barrier2(
                        self.image_transfer_command_buffer,
                        &vk::DependencyInfoKHR::builder()
                            .image_memory_barriers(&image_barriers)
                            .build(),
                    );
                }

                // コマンドバッファの終了
                unsafe {
                    self.device
                        .end_command_buffer(self.image_transfer_command_buffer)
                }?;

                // コマンドバッファのサブミット
                let buffers_to_submit = [self.image_transfer_command_buffer];
                let submit_info = vk::SubmitInfo::builder()
                    .command_buffers(&buffers_to_submit)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])
                    .wait_semaphores(&[])
                    .build();
                unsafe {
                    self.device.queue_submit(
                        self.transfer_queue,
                        &[submit_info],
                        vk::Fence::null(),
                    )?;
                }

                // コマンド完了まで待機
                unsafe { self.device.device_wait_idle()? };
            }

            (image, image_memory, image_view)
        };
        self.storage_image = storage_image;
        self.storage_image_memory = storage_image_memory;
        self.storage_image_view = storage_image_view;

        // descriptor setの作成
        let descriptor_set = {
            // descriptor setのアロケート
            let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = unsafe {
                self.device
                    .allocate_descriptor_sets(&descriptor_set_allocate_info)
            }?[0];

            // acceleration structureの書き込み
            let mut descriptor_acceleration_structure_info =
                vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                    .acceleration_structures(std::slice::from_ref(&self.tlas));
            let mut acceleration_structure_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .push_next(&mut descriptor_acceleration_structure_info);
            acceleration_structure_write.descriptor_count = 1;

            // storage imageの書き込み
            let storage_image_info = vk::DescriptorImageInfo::builder()
                .image_view(storage_image_view)
                .image_layout(vk::ImageLayout::GENERAL);
            let storage_image_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&storage_image_info));

            // descriptor setの更新
            let descriptor_writes = [
                acceleration_structure_write.build(),
                storage_image_write.build(),
            ];
            unsafe { self.device.update_descriptor_sets(&descriptor_writes, &[]) };

            descriptor_set
        };
        self.descriptor_set = descriptor_set;

        // sync objectsの作成
        // fenceの作成
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let mut in_flight_fences = vec![];
        for _ in 0..self.swapchain_images.len() {
            let fence = unsafe { self.device.create_fence(&fence_create_info, None)? };
            in_flight_fences.push(fence);
        }
        self.in_flight_fences = in_flight_fences;

        // semaphoreの作成
        let mut image_available_semaphores = vec![];
        for _ in 0..self.swapchain_images.len() {
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let timeline_semaphore =
                unsafe { self.device.create_semaphore(&semaphore_create_info, None)? };
            image_available_semaphores.push(timeline_semaphore);
        }
        let mut render_finished_semaphores = vec![];
        for _ in 0..self.swapchain_images.len() {
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let timeline_semaphore =
                unsafe { self.device.create_semaphore(&semaphore_create_info, None)? };
            render_finished_semaphores.push(timeline_semaphore);
        }
        self.image_available_semaphores = image_available_semaphores;
        self.render_finished_semaphores = render_finished_semaphores;

        // dirty flagを解除する
        self.dirty_swapchain = false;

        Ok(())
    }

    fn render(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        if self.dirty_swapchain {
            self.recreate_swapchain(width, height)?;
        }

        // swapchainから次のimageを取得
        let result = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                std::u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            )
        };
        let index = match result {
            Ok((index, _)) => index as usize,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.dirty_swapchain = true;
                return Ok(());
            }
            Err(error) => return Err(anyhow::anyhow!(error)),
        };

        // fenceを待機
        unsafe {
            self.device.wait_for_fences(
                std::slice::from_ref(&self.in_flight_fences[self.current_frame]),
                true,
                u64::MAX,
            )
        }?;

        // fenceをリセット
        unsafe {
            self.device.reset_fences(std::slice::from_ref(
                &self.in_flight_fences[self.current_frame],
            ))
        }?;

        // コマンドバッファのクリア
        unsafe {
            self.device.reset_command_buffer(
                self.render_command_buffers[self.current_frame],
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )?;
        }

        // コマンドバッファの記録開始
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device.begin_command_buffer(
                self.render_command_buffers[self.current_frame],
                &command_buffer_begin_info,
            )?
        }

        // sbt entryの用意
        let raygen_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(self.raygen_sbt_device_address)
            .stride(self.raygen_sbt_stride)
            .size(self.raygen_sbt_size);
        let miss_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(self.miss_sbt_device_address)
            .stride(self.miss_sbt_stride)
            .size(self.miss_sbt_size);
        let hit_shader_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(self.hit_sbt_device_address)
            .stride(self.hit_sbt_stride)
            .size(self.hit_sbt_size);

        // raytracing pipelineのbind
        unsafe {
            self.device.cmd_bind_pipeline(
                self.render_command_buffers[self.current_frame],
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.raytracing_pipeline,
            )
        };

        // descriptor setのbind
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.render_command_buffers[self.current_frame],
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.raytracing_pipeline_layout,
                0,
                std::slice::from_ref(&self.descriptor_set),
                &[],
            )
        };

        // raytracingの実行
        unsafe {
            self.raytracing_pipeline_loader.cmd_trace_rays(
                self.render_command_buffers[self.current_frame],
                &raygen_shader_sbt_entry,
                &miss_shader_sbt_entry,
                &hit_shader_sbt_entry,
                &vk::StridedDeviceAddressRegionKHR::default(),
                self.width,
                self.height,
                1,
            )
        };

        // swapchain imageのレイアウトをコピー先に変更
        let swapchain_image_barriers = vk::ImageMemoryBarrier2::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags2KHR::empty())
            .old_layout(vk::ImageLayout::UNDEFINED)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::TRANSFER)
            .dst_access_mask(vk::AccessFlags2KHR::TRANSFER_WRITE)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image(self.swapchain_images[index]);
        unsafe {
            self.device.cmd_pipeline_barrier2(
                self.render_command_buffers[self.current_frame],
                &vk::DependencyInfoKHR::builder()
                    .image_memory_barriers(std::slice::from_ref(&swapchain_image_barriers)),
            );
        }

        // storage imageのレイアウトをコピー元に変更
        let storage_image_barriers = vk::ImageMemoryBarrier2::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags2KHR::empty())
            .old_layout(vk::ImageLayout::UNDEFINED)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::TRANSFER)
            .dst_access_mask(vk::AccessFlags2KHR::TRANSFER_READ)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image(self.storage_image);
        unsafe {
            self.device.cmd_pipeline_barrier2(
                self.render_command_buffers[self.current_frame],
                &vk::DependencyInfoKHR::builder()
                    .image_memory_barriers(std::slice::from_ref(&storage_image_barriers)),
            );
        }

        // storage imageをswapchain imageにコピー
        let copy_region = vk::ImageCopy2::builder()
            .src_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )
            .dst_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )
            .extent(
                vk::Extent3D::builder()
                    .width(self.width)
                    .height(self.height)
                    .depth(1)
                    .build(),
            );
        let copy_image_info = vk::CopyImageInfo2KHR::builder()
            .src_image(self.storage_image)
            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .dst_image(self.swapchain_images[index])
            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .regions(std::slice::from_ref(&copy_region));
        unsafe {
            self.device.cmd_copy_image2(
                self.render_command_buffers[self.current_frame],
                &copy_image_info,
            );
        }

        // swapchain imageのレイアウトを表示用に変更
        let swapchain_image_barriers = vk::ImageMemoryBarrier2::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::TRANSFER)
            .src_access_mask(vk::AccessFlags2KHR::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2KHR::empty())
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image(self.swapchain_images[index]);
        unsafe {
            self.device.cmd_pipeline_barrier2(
                self.render_command_buffers[self.current_frame],
                &vk::DependencyInfoKHR::builder()
                    .image_memory_barriers(std::slice::from_ref(&swapchain_image_barriers)),
            );
        }

        // storage imageのレイアウトをGeneralに戻す
        let storage_image_barriers = vk::ImageMemoryBarrier2::builder()
            .src_stage_mask(vk::PipelineStageFlags2KHR::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags2KHR::empty())
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .dst_stage_mask(vk::PipelineStageFlags2KHR::TOP_OF_PIPE)
            .dst_access_mask(vk::AccessFlags2KHR::empty())
            .new_layout(vk::ImageLayout::GENERAL)
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image(self.storage_image);
        unsafe {
            self.device.cmd_pipeline_barrier2(
                self.render_command_buffers[self.current_frame],
                &vk::DependencyInfoKHR::builder()
                    .image_memory_barriers(std::slice::from_ref(&storage_image_barriers)),
            );
        }

        // コマンドバッファの終了
        unsafe {
            self.device
                .end_command_buffer(self.render_command_buffers[self.current_frame])
        }?;

        // コマンドバッファのサブミット
        let buffers_to_submit = [self.render_command_buffers[self.current_frame]];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&buffers_to_submit)
            .wait_semaphores(std::slice::from_ref(
                &self.image_available_semaphores[self.current_frame],
            ))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])
            .signal_semaphores(std::slice::from_ref(
                &self.render_finished_semaphores[self.current_frame],
            ));
        unsafe {
            self.device.queue_submit(
                self.graphics_queue,
                std::slice::from_ref(&submit_info),
                self.in_flight_fences[self.current_frame],
            )?;
        };

        // swapchain imageをpresentする
        let image_indices = [index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(
                &self.render_finished_semaphores[self.current_frame],
            ))
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(&image_indices);
        let result = unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
        };
        let is_dirty_swapchain = match result {
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => true,
            Err(error) => panic!("Failed to present queue. Cause: {}", error),
            _ => false,
        };
        self.dirty_swapchain = is_dirty_swapchain;

        // current_frameを更新
        self.current_frame = (self.current_frame + 1) % self.in_flight_fences.len();

        Ok(())
    }

    pub fn run() -> Result<()> {
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
            .build(&event_loop)?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = Self::init(&event_loop, &window)?;

        event_loop.run(move |event, elwt| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(_) => {
                    app.dirty_swapchain = true;
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    app.render(size.width, size.height).unwrap();
                }
                _ => (),
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => (),
        })?;
        Ok(())
    }
}
impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            for &fence in self.in_flight_fences.iter() {
                self.device.destroy_fence(fence, None);
            }
            for &semaphore in self.image_available_semaphores.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in self.render_finished_semaphores.iter() {
                self.device.destroy_semaphore(semaphore, None);
            }

            self.device
                .free_command_buffers(self.graphics_command_pool, &self.render_command_buffers);

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            self.device.destroy_buffer(self.sbt_buffer, None);
            self.device.free_memory(self.sbt_buffer_memory, None);

            self.device.destroy_pipeline(self.raytracing_pipeline, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device
                .destroy_pipeline_layout(self.raytracing_pipeline_layout, None);

            self.acceleration_structure_loader
                .destroy_acceleration_structure(self.tlas, None);
            self.device.destroy_buffer(self.tlas_buffer, None);
            self.device.free_memory(self.tlas_memory, None);

            self.acceleration_structure_loader
                .destroy_acceleration_structure(self.blas, None);
            self.device.destroy_buffer(self.blas_buffer, None);
            self.device.free_memory(self.blas_memory, None);

            self.device.free_command_buffers(
                self.graphics_command_pool,
                &[self.image_transfer_command_buffer],
            );

            self.device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.device
                .destroy_image_view(self.storage_image_view, None);
            self.device.destroy_image(self.storage_image, None);
            self.device.free_memory(self.storage_image_memory, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);
            if Self::ENABLE_VALIDATION_LAYERS {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
