use ash::vk;

/// Fenceを作成する関数
pub fn create_fence(device: &crate::DeviceHandle) -> crate::FenceHandle {
    let create_info = vk::FenceCreateInfo::builder();
    device.create_fence(&create_info)
}

/// シグナル状態のFenceを作成する関数
pub fn create_signaled_fence(device: &crate::DeviceHandle) -> crate::FenceHandle {
    let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    device.create_fence(&create_info)
}
