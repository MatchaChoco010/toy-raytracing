//! 参照カウンタで管理して、参照がすべて破棄された際に
//! DescriptorSetLayoutの破棄の処理まで行うDescriptorSetLayoutHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct DescriptorSetLayoutHandleData {
    device: crate::DeviceHandle,
    descriptor_set_layout: vk::DescriptorSetLayout,
    ref_count: AtomicUsize,
}
impl DescriptorSetLayoutHandleData {
    fn new(
        device: crate::DeviceHandle,
        descriptor_set_layout_create_info: &vk::DescriptorSetLayoutCreateInfo,
    ) -> Result<Self> {
        // create descriptor set layout
        let descriptor_set_layout = unsafe {
            ash::Device::create_descriptor_set_layout(
                &device,
                descriptor_set_layout_create_info,
                None,
            )?
        };

        Ok(Self {
            device,
            descriptor_set_layout,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::DescriptorSetLayoutを参照カウントで管理するためのハンドル
pub struct DescriptorSetLayoutHandle {
    ptr: NonNull<DescriptorSetLayoutHandleData>,
}
impl DescriptorSetLayoutHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        descriptor_set_layout_create_info: &vk::DescriptorSetLayoutCreateInfo,
    ) -> Self {
        let data = Box::new(
            DescriptorSetLayoutHandleData::new(device_handle, descriptor_set_layout_create_info)
                .expect("Failed to create descriptor set layout."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::DescriptorSetLayoutを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::DescriptorSetLayoutは無効になる。
    pub unsafe fn descriptor_set_layout_raw(&self) -> vk::DescriptorSetLayout {
        self.data().descriptor_set_layout.clone()
    }

    fn data(&self) -> &DescriptorSetLayoutHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for DescriptorSetLayoutHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorSetLayoutHandle").finish()
    }
}

// DescriptorSetLayoutHandleDataの中身はSendかつSyncなのでDescriptorSetLayoutHandleはSend
unsafe impl Send for DescriptorSetLayoutHandle {}
// DescriptorSetLayoutHandleDataの中身はSendかつSyncなのでDescriptorSetLayoutHandleはSync
unsafe impl Sync for DescriptorSetLayoutHandle {}

// DescriptorSetLayoutHandleはvk::DescriptorSetLayoutにDerefする
impl Deref for DescriptorSetLayoutHandle {
    type Target = vk::DescriptorSetLayout;
    fn deref(&self) -> &Self::Target {
        &self.data().descriptor_set_layout
    }
}

// Cloneで参照カウントを増やす
impl Clone for DescriptorSetLayoutHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to DescriptorSetLayoutHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for DescriptorSetLayoutHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // descriptor set layoutの破棄
                data.device
                    .destroy_descriptor_set_layout(data.descriptor_set_layout, None);
            }
        }
    }
}
