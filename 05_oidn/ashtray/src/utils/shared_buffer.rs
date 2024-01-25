use ash::vk;
#[cfg(target_os = "windows")]
use std::ffi::c_void;

/// 外部プログラムと共有できるGPUメモリのバッファ
pub struct SharedBuffer {
    device: crate::DeviceHandle,
    /// BufferHandle
    pub buffer: crate::BufferHandle,
    /// vk::DeviceMemory
    pub memory: vk::DeviceMemory,
    /// Bufferのデバイスアドレス
    pub device_address: u64,
    /// bufferのsize
    pub size: u64,

    /// handle
    #[cfg(target_os = "windows")]
    pub handle: *mut c_void,
    /// fd
    #[cfg(target_os = "linux")]
    pub fd: i32,
}
impl SharedBuffer {
    /// SharedBufferを作成する
    pub fn new(
        device: &crate::DeviceHandle,
        buffer_size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        // bufferの作成
        #[cfg(target_os = "windows")]
        let mut external_memory_buffer_create_info = vk::ExternalMemoryBufferCreateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);
        #[cfg(target_os = "linux")]
        let mut external_memory_buffer_create_info = vk::ExternalMemoryBufferCreateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(usage)
            .push_next(&mut external_memory_buffer_create_info);
        let buffer = device.create_buffer(&buffer_create_info);

        // memoryの確保
        let buffer_memory_requirement = buffer.get_buffer_memory_requirements();

        let physical_device_memory_properties = device.get_physical_device_memory_properties();
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
            .expect("No suitable memory type") as u32;

        #[cfg(target_os = "windows")]
        let mut export_memory_allocate_info = vk::ExportMemoryAllocateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);
        #[cfg(target_os = "linux")]
        let mut export_memory_allocate_info = vk::ExportMemoryAllocateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);

        let allocation_flags = vk::MemoryAllocateFlags::DEVICE_ADDRESS;
        let mut flags_info = vk::MemoryAllocateFlagsInfo::builder().flags(allocation_flags);

        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_memory_requirement.size)
            .memory_type_index(memory_type_index)
            .push_next(&mut export_memory_allocate_info)
            .push_next(&mut flags_info);
        let memory = unsafe {
            device
                .allocate_memory(&memory_allocate_info, None)
                .expect("shared memory allocation error")
        };

        // bufferとメモリのバインド
        buffer.bind_buffer_memory(memory, 0);

        // device addressの取得
        let device_address = device
            .get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(*buffer));

        #[cfg(target_os = "windows")]
        let handle = device.get_memory_win32_handle(
            &vk::MemoryGetWin32HandleInfoKHR::builder()
                .memory(memory)
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32),
        );

        #[cfg(target_os = "linux")]
        let fd = device.get_memory_fd(
            &vk::MemoryGetFdInfoKHR::builder()
                .memory(memory)
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD_KHR),
        );

        Self {
            device: device.clone(),
            buffer,
            memory,
            device_address,
            size: buffer_size,
            #[cfg(target_os = "windows")]
            handle,
            #[cfg(target_os = "linux")]
            fd,
        }
    }
}
impl Drop for SharedBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
        }
    }
}
