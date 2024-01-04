//! 参照カウンタで管理して、参照がすべて破棄された際に
//! DescriptorPoolの破棄の処理まで行うDescriptorPoolHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct DescriptorPoolHandleData {
    device: crate::DeviceHandle,
    descriptor_pool: vk::DescriptorPool,
    ref_count: AtomicUsize,
}
impl DescriptorPoolHandleData {
    fn new(
        device: crate::DeviceHandle,
        descriptor_pool_create_info: &vk::DescriptorPoolCreateInfo,
    ) -> Result<Self> {
        // create descriptor pool
        let descriptor_pool = unsafe {
            ash::Device::create_descriptor_pool(&device, descriptor_pool_create_info, None)?
        };

        Ok(Self {
            device,
            descriptor_pool,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::DescriptorPoolを参照カウントで管理するためのハンドル
pub struct DescriptorPoolHandle {
    ptr: NonNull<DescriptorPoolHandleData>,
}
impl DescriptorPoolHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        descriptor_pool_create_info: &vk::DescriptorPoolCreateInfo,
    ) -> Self {
        let data = Box::new(
            DescriptorPoolHandleData::new(device_handle, descriptor_pool_create_info)
                .expect("Failed to create descriptor pool."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn descriptor_pool_raw(&self) -> vk::DescriptorPool {
        self.data().descriptor_pool.clone()
    }

    fn data(&self) -> &DescriptorPoolHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for DescriptorPoolHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorPoolHandle").finish()
    }
}

// DescriptorPoolHandleDataの中身はSendかつSyncなのでDescriptorPoolHandleはSend
unsafe impl Send for DescriptorPoolHandle {}
// DescriptorPoolHandleDataの中身はSendかつSyncなのでDescriptorPoolHandleはSync
unsafe impl Sync for DescriptorPoolHandle {}

// DescriptorPoolHandleはvk::DescriptorPoolにDerefする
impl Deref for DescriptorPoolHandle {
    type Target = vk::DescriptorPool;
    fn deref(&self) -> &Self::Target {
        &self.data().descriptor_pool
    }
}

// Cloneで参照カウントを増やす
impl Clone for DescriptorPoolHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to DescriptorPoolHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for DescriptorPoolHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // descriptor poolの破棄
                data.device
                    .destroy_descriptor_pool(data.descriptor_pool, None);
            }
        }
    }
}
