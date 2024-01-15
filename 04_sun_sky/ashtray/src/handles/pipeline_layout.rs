//! 参照カウンタで管理して、参照がすべて破棄された際に
//! PipelineLayoutの破棄の処理まで行うPipelineLayoutHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct PipelineLayoutHandleData {
    device: crate::DeviceHandle,
    pipeline_layout: vk::PipelineLayout,
    ref_count: AtomicUsize,
}
impl PipelineLayoutHandleData {
    fn new(
        device: crate::DeviceHandle,
        pipeline_layout_create_info: &vk::PipelineLayoutCreateInfo,
    ) -> Result<Self> {
        // create pipeline layout
        let pipeline_layout = unsafe {
            ash::Device::create_pipeline_layout(&device, pipeline_layout_create_info, None)?
        };

        Ok(Self {
            device,
            pipeline_layout,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::PipelineLayoutを参照カウントで管理するためのハンドル
pub struct PipelineLayoutHandle {
    ptr: NonNull<PipelineLayoutHandleData>,
}
impl PipelineLayoutHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        pipeline_layout_create_info: &vk::PipelineLayoutCreateInfo,
    ) -> Self {
        let data = Box::new(
            PipelineLayoutHandleData::new(device_handle, pipeline_layout_create_info)
                .expect("Failed to create pipeline layout."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::PipelineLayoutを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::PipelineLayoutは無効になる。
    pub unsafe fn pipeline_layout_raw(&self) -> vk::PipelineLayout {
        self.data().pipeline_layout.clone()
    }

    fn data(&self) -> &PipelineLayoutHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for PipelineLayoutHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineLayoutHandle").finish()
    }
}

// PipelineLayoutHandleDataの中身はSendかつSyncなのでPipelineLayoutHandleはSend
unsafe impl Send for PipelineLayoutHandle {}
// PipelineLayoutHandleDataの中身はSendかつSyncなのでPipelineLayoutHandleはSync
unsafe impl Sync for PipelineLayoutHandle {}

// PipelineLayoutHandleはvk::PipelineLayoutにDerefする
impl Deref for PipelineLayoutHandle {
    type Target = vk::PipelineLayout;
    fn deref(&self) -> &Self::Target {
        &self.data().pipeline_layout
    }
}

// Cloneで参照カウントを増やす
impl Clone for PipelineLayoutHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to PipelineLayoutHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for PipelineLayoutHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // pipeline layoutの破棄
                data.device
                    .destroy_pipeline_layout(data.pipeline_layout, None);
            }
        }
    }
}
