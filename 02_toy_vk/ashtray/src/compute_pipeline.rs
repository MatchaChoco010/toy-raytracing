//! 参照カウンタで管理して、参照がすべて破棄された際に
//! ComputePipelineの破棄の処理まで行うComputePipelineHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ComputePipelineHandleData {
    device: crate::DeviceHandle,
    pipeline_layout: crate::PipelineLayoutHandle,
    compute_pipeline: vk::Pipeline,
    ref_count: AtomicUsize,
}
impl ComputePipelineHandleData {
    fn new(
        device: crate::DeviceHandle,
        pipeline_cache: vk::PipelineCache,
        pipeline_layout: crate::PipelineLayoutHandle,
        compute_pipeline_create_infos: &[vk::ComputePipelineCreateInfo],
    ) -> Result<Vec<Self>> {
        // create compute pipeline
        let compute_pipelines = unsafe {
            ash::Device::create_compute_pipelines(
                &device,
                pipeline_cache,
                compute_pipeline_create_infos,
                None,
            )
            .expect("Failed to create compute pipeline.")
        };

        let compute_pipelines = compute_pipelines
            .into_iter()
            .map(|compute_pipeline| Self {
                device: device.clone(),
                pipeline_layout: pipeline_layout.clone(),
                compute_pipeline,
                ref_count: AtomicUsize::new(1),
            })
            .collect::<Vec<_>>();

        Ok(compute_pipelines)
    }
}

/// vk::Pipelineを参照カウントで管理するためのハンドル
pub struct ComputePipelineHandle {
    ptr: NonNull<ComputePipelineHandleData>,
}
impl ComputePipelineHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        pipeline_cache: vk::PipelineCache,
        pipeline_layout_handle: crate::PipelineLayoutHandle,
        compute_pipeline_create_infos: &[vk::ComputePipelineCreateInfo],
    ) -> Vec<Self> {
        let data = Box::new(
            ComputePipelineHandleData::new(
                device_handle,
                pipeline_cache,
                pipeline_layout_handle,
                compute_pipeline_create_infos,
            )
            .expect("Failed to create compute pipeline."),
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

    pub fn pipeline_layout(&self) -> crate::PipelineLayoutHandle {
        self.data().pipeline_layout.clone()
    }

    pub unsafe fn compute_pipeline_raw(&self) -> vk::Pipeline {
        self.data().compute_pipeline.clone()
    }

    fn data(&self) -> &ComputePipelineHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for ComputePipelineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputePipelineHandle").finish()
    }
}

// ComputePipelineHandleDataの中身はSendかつSyncなのでComputePipelineHandleはSend
unsafe impl Send for ComputePipelineHandle {}
// ComputePipelineHandleDataの中身はSendかつSyncなのでComputePipelineHandleはSync
unsafe impl Sync for ComputePipelineHandle {}

// ComputePipelineHandleはvk::PipelineにDerefする
impl Deref for ComputePipelineHandle {
    type Target = vk::Pipeline;
    fn deref(&self) -> &Self::Target {
        &self.data().compute_pipeline
    }
}

// Cloneで参照カウントを増やす
impl Clone for ComputePipelineHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to ComputePipelineHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for ComputePipelineHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // compute pipelineの破棄
                data.device.destroy_pipeline(data.compute_pipeline, None);
            }
        }
    }
}
