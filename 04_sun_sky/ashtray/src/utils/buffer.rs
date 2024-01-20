use crate::utils::*;
use ash::vk;

/// Bufferの関連するオブジェクトをまとめた構造体
#[derive(Clone)]
pub struct BufferObjects {
    /// BufferHandle
    pub buffer: crate::BufferHandle,
    /// AllocationHandle
    pub allocation: crate::AllocationHandle,
    /// Bufferのデバイスアドレス
    pub device_address: u64,
}

/// HostのBufferを作成する関数
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

/// HostのBufferを作成し、データをコピーする関数
pub fn create_host_buffer_with_data<T: Copy>(
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
    let mut allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "host buffer",
        requirements: buffer_memory_requirement,
        location: gpu_allocator::MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    });

    // bufferとメモリのバインド
    buffer.bind_buffer_memory(allocation.memory(), allocation.offset());

    // データのコピー
    presser::copy_from_slice_to_offset_with_align(data, &mut *allocation, 0, 4).unwrap();
    // let ptr = allocation.mapped_ptr().unwrap().as_ptr();
    // unsafe {
    //     std::ptr::copy_nonoverlapping(
    //         data.as_ptr() as *const u8,
    //         ptr as *mut u8,
    //         buffer_size as usize,
    //     )
    // };

    // device addressの取得
    let device_address =
        device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}

/// DeviceLocalのBufferを作成する関数
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

/// DeviceLocalのBufferを作成し、データをコピーする関数
pub fn create_device_local_buffer_with_data<T: Copy>(
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
        std::slice::from_ref(
            &vk::BufferCopy::builder()
                .size(buffer_size)
                .src_offset(0)
                .dst_offset(0),
        ),
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
    device.wait_idle();

    BufferObjects {
        buffer,
        allocation,
        device_address,
    }
}
