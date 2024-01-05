use crate::utils::*;
use ash::vk;

/// Blas関連のオブジェクトをまとめた構造体
#[derive(Clone)]
pub struct BlasObjects {
    /// BlasのAccelerationStructureHandle
    pub blas: crate::AccelerationStructureHandle,
    /// BlasのBufferObjects
    pub blas_buffer: BufferObjects,
    /// BlasのVertexBuffer
    pub vertex_buffer: BufferObjects,
    /// BlasのIndexBuffer
    pub index_buffer: BufferObjects,
}

/// Blasを作成するヘルパー関数
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

/// Tlas関連のオブジェクトをまとめた構造体
#[derive(Clone)]
pub struct TlasObjects {
    /// TlasのAccelerationStructureHandle
    pub tlas: crate::AccelerationStructureHandle,
    /// TlasのBufferObjects
    pub tlas_buffer: BufferObjects,
    /// TlasのInstanceParamのBufferObjects
    pub instance_params_buffer: BufferObjects,
    /// TlasのMaterialのBufferObjects
    pub materials_buffer: BufferObjects,
}

/// Tlasを作成するヘルパー関数
pub fn create_tlas<Material>(
    device: &crate::DeviceHandle,
    queue_handles: &QueueHandles,
    compute_command_pool: &crate::CommandPoolHandle,
    transfer_command_pool: &crate::CommandPoolHandle,
    allocator: &crate::AllocatorHandle,
    instances: &[(BlasObjects, glam::Mat4, u32)],
    materials: &[Material],
) -> TlasObjects {
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    struct InstanceParam {
        pub address_index: u64,
        pub address_vertex: u64,
        pub transform: glam::Mat4,
        pub material_index: u32,
        pub padding_1: u32,
        pub padding_2: u64,
    }

    // instancesを作成
    let instances_data = instances
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

    // instancesのbufferを作成
    let instances_buffer = create_host_buffer_with_data(
        &device,
        &allocator,
        &instances_data,
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
    let primitive_count = instances.len() as u32;
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
    let instance_params = instances
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

/// HitShaderGroupのShaderModuleをまとめた構造体
pub struct HitShaderModules {
    /// ClosestHitShaderのShaderModuleHandle
    pub closest_hit: crate::ShaderModuleHandle,
    /// AnyHitShaderのShaderModuleHandle
    pub any_hit: Option<crate::ShaderModuleHandle>,
    /// IntersectionShaderのShaderModuleHandle
    pub intersection: Option<crate::ShaderModuleHandle>,
}

/// RayTracingPipelineを作成するヘルパー関数
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
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR);

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

/// ShaderBindingTableのItemをまとめた構造体
pub struct SbtItem {
    /// ShaderBindingTableの要素のdevice address
    pub device_address: u64,
    /// ShaderBindingTableの要素のstride
    pub stride: u64,
    /// ShaderBindingTableの要素のsize
    pub size: u64,
}

/// ShaderBindingTableをまとめた構造体
pub struct ShaderBindingTable {
    /// ShaderBindingTableのBufferObjects
    pub buffer: BufferObjects,
    /// RaygenShaderGroupのSbtItem
    pub raygen_item: SbtItem,
    /// MissShaderGroupのSbtItem
    pub miss_item: SbtItem,
    /// HitShaderGroupのSbtItem
    pub hit_item: SbtItem,
}

/// ShaderBindingTableを作成するヘルパー関数
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
