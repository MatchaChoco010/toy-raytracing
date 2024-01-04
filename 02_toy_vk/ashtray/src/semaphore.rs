//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Semaphoreの破棄の処理まで行うSemaphoreHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct SemaphoreHandleData {
    device: crate::DeviceHandle,
    semaphore: vk::Semaphore,
    ref_count: AtomicUsize,
}
impl SemaphoreHandleData {
    fn new(
        device: crate::DeviceHandle,
        semaphore_create_info: &vk::SemaphoreCreateInfo,
    ) -> Result<Self> {
        // create Semaphore
        let semaphore =
            unsafe { ash::Device::create_semaphore(&device, semaphore_create_info, None)? };

        Ok(Self {
            device,
            semaphore,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Semaphoreを参照カウントで管理するためのハンドル
pub struct SemaphoreHandle {
    ptr: NonNull<SemaphoreHandleData>,
}
impl SemaphoreHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        semaphore_create_info: &vk::SemaphoreCreateInfo,
    ) -> Self {
        let data = SemaphoreHandleData::new(device_handle, semaphore_create_info)
            .expect("Failed to create Semaphore.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn semaphore_raw(&self) -> vk::Semaphore {
        self.data().semaphore.clone()
    }

    fn data(&self) -> &SemaphoreHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for SemaphoreHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemaphoreHandle").finish()
    }
}

// SemaphoreHandleDataの中身はSendかつSyncなのでSemaphoreHandleはSend
unsafe impl Send for SemaphoreHandle {}
// SemaphoreHandleDataの中身はSendかつSyncなのでSemaphoreHandleはSync
unsafe impl Sync for SemaphoreHandle {}

// SemaphoreHandleはvk::ImageにDerefする
impl Deref for SemaphoreHandle {
    type Target = vk::Semaphore;
    fn deref(&self) -> &Self::Target {
        &self.data().semaphore
    }
}

// Cloneで参照カウントを増やす
impl Clone for SemaphoreHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to SemaphoreHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for SemaphoreHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // Semaphoreの破棄
                data.device.destroy_semaphore(data.semaphore, None);
            }
        }
    }
}
