//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Swapchainの破棄の処理まで行うSwapchainHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct SwapchainHandleData {
    device: crate::DeviceHandle,
    swapchain: vk::SwapchainKHR,
    ref_count: AtomicUsize,
}
impl SwapchainHandleData {
    fn new(
        device: crate::DeviceHandle,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
    ) -> Result<Self> {
        // create swapchain
        let swapchain = unsafe {
            device
                .swapchain_loader_raw()
                .create_swapchain(swapchain_create_info, None)?
        };

        Ok(Self {
            device,
            swapchain,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::SwapchainKHRを参照カウントで管理するためのハンドル
pub struct SwapchainHandle {
    ptr: NonNull<SwapchainHandleData>,
}
impl SwapchainHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
    ) -> Self {
        let data = SwapchainHandleData::new(device_handle, swapchain_create_info)
            .expect("Failed to create swapchain.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::SwapchainKHRを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::SwapchainKHRは無効になる。
    pub unsafe fn swapchain_raw(&self) -> vk::SwapchainKHR {
        self.data().swapchain.clone()
    }

    fn data(&self) -> &SwapchainHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for SwapchainHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapchainHandle").finish()
    }
}

// SwapchainHandleDataの中身はSendかつSyncなのでSwapchainHandleはSend
unsafe impl Send for SwapchainHandle {}
// SwapchainHandleDataの中身はSendかつSyncなのでSwapchainHandleはSync
unsafe impl Sync for SwapchainHandle {}

// SwapchainHandleはvk::SwapchainにDerefする
impl Deref for SwapchainHandle {
    type Target = vk::SwapchainKHR;
    fn deref(&self) -> &Self::Target {
        &self.data().swapchain
    }
}

// Cloneで参照カウントを増やす
impl Clone for SwapchainHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to SwapchainHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for SwapchainHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // imageの破棄
                data.device
                    .swapchain_loader_raw()
                    .destroy_swapchain(data.swapchain, None);
            }
        }
    }
}
