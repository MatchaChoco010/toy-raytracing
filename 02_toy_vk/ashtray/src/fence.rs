//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Fenceの破棄の処理まで行うFenceHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct FenceHandleData {
    device: crate::DeviceHandle,
    fence: vk::Fence,
    ref_count: AtomicUsize,
}
impl FenceHandleData {
    fn new(device: crate::DeviceHandle, fence_create_info: &vk::FenceCreateInfo) -> Result<Self> {
        // create Fence
        let fence = unsafe { ash::Device::create_fence(&device, fence_create_info, None)? };

        Ok(Self {
            device,
            fence,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Fenceを参照カウントで管理するためのハンドル
pub struct FenceHandle {
    ptr: NonNull<FenceHandleData>,
}
impl FenceHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        fence_create_info: &vk::FenceCreateInfo,
    ) -> Self {
        let data = FenceHandleData::new(device_handle, fence_create_info)
            .expect("Failed to create Fence.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn fence_raw(&self) -> vk::Fence {
        self.data().fence.clone()
    }

    fn data(&self) -> &FenceHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for FenceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FenceHandle").finish()
    }
}

// FenceHandleDataの中身はSendかつSyncなのでFenceHandleはSend
unsafe impl Send for FenceHandle {}
// FenceHandleDataの中身はSendかつSyncなのでFenceHandleはSync
unsafe impl Sync for FenceHandle {}

// FenceHandleはvk::ImageにDerefする
impl Deref for FenceHandle {
    type Target = vk::Fence;
    fn deref(&self) -> &Self::Target {
        &self.data().fence
    }
}

// Cloneで参照カウントを増やす
impl Clone for FenceHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to FenceHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for FenceHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // Fenceの破棄
                data.device.destroy_fence(data.fence, None);
            }
        }
    }
}
