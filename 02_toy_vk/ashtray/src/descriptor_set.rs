//! 参照カウンタで管理して、参照がすべて破棄された際に
//! DescriptorSetの破棄の処理まで行うDescriptorSetHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct DescriptorSetHandleData {
    device: crate::DeviceHandle,
    descriptor_pool: crate::DescriptorPoolHandle,
    descriptor_set_layout: crate::DescriptorSetLayoutHandle,
    descriptor_set: vk::DescriptorSet,
    ref_count: AtomicUsize,
}
impl DescriptorSetHandleData {
    fn new(
        device: crate::DeviceHandle,
        descriptor_pool: &crate::DescriptorPoolHandle,
        descriptor_set_layout: &crate::DescriptorSetLayoutHandle,
        descriptor_set_allocate_info: &vk::DescriptorSetAllocateInfo,
    ) -> Result<Vec<Self>> {
        // create descriptor_set
        let descriptor_sets = unsafe {
            ash::Device::allocate_descriptor_sets(&device, descriptor_set_allocate_info)?
        };

        let descriptor_sets = descriptor_sets
            .into_iter()
            .map(|descriptor_set| Self {
                device: device.clone(),
                descriptor_pool: descriptor_pool.clone(),
                descriptor_set_layout: descriptor_set_layout.clone(),
                descriptor_set,
                ref_count: AtomicUsize::new(1),
            })
            .collect();

        Ok(descriptor_sets)
    }
}

/// vk::DescriptorSetを参照カウントで管理するためのハンドル
pub struct DescriptorSetHandle {
    ptr: NonNull<DescriptorSetHandleData>,
}
impl DescriptorSetHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        descriptor_pool: &crate::DescriptorPoolHandle,
        descriptor_set_layout_handle: &crate::DescriptorSetLayoutHandle,
        descriptor_set_allocate_info: &vk::DescriptorSetAllocateInfo,
    ) -> Vec<Self> {
        let data = Box::new(
            DescriptorSetHandleData::new(
                device_handle,
                descriptor_pool,
                descriptor_set_layout_handle,
                descriptor_set_allocate_info,
            )
            .expect("Failed to create descriptor_set."),
        );
        let ptrs = data
            .into_iter()
            .map(|data| unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) })
            .collect::<Vec<_>>();
        ptrs.into_iter().map(|ptr| Self { ptr }).collect()
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }
    pub fn descriptor_pool(&self) -> crate::DescriptorPoolHandle {
        self.data().descriptor_pool.clone()
    }

    pub fn descriptor_set_layout(&self) -> crate::DescriptorSetLayoutHandle {
        self.data().descriptor_set_layout.clone()
    }

    pub unsafe fn descriptor_set_raw(&self) -> vk::DescriptorSet {
        self.data().descriptor_set.clone()
    }

    fn data(&self) -> &DescriptorSetHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for DescriptorSetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorSetHandle").finish()
    }
}

// DescriptorSetHandleDataの中身はSendかつSyncなのでDescriptorSetHandleはSend
unsafe impl Send for DescriptorSetHandle {}
// DescriptorSetHandleDataの中身はSendかつSyncなのでDescriptorSetHandleはSync
unsafe impl Sync for DescriptorSetHandle {}

// DescriptorSetHandleはvk::DescriptorSetにDerefする
impl Deref for DescriptorSetHandle {
    type Target = vk::DescriptorSet;
    fn deref(&self) -> &Self::Target {
        &self.data().descriptor_set
    }
}

// Cloneで参照カウントを増やす
impl Clone for DescriptorSetHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to DescriptorSetHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for DescriptorSetHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // descriptor_setの破棄
                data.device
                    .free_descriptor_sets(*data.descriptor_pool, &[data.descriptor_set])
                    .expect("Failed to free descriptor set.");
            }
        }
    }
}
