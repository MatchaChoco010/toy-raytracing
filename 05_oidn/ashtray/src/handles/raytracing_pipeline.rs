//! 参照カウンタで管理して、参照がすべて破棄された際に
//! RayTracingPipelineの破棄の処理まで行うRayTracingPipelineHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct RayTracingPipelineHandleData {
    device: crate::DeviceHandle,
    ray_tracing_pipeline: vk::Pipeline,
    ref_count: AtomicUsize,
}
impl RayTracingPipelineHandleData {
    fn new(
        device: crate::DeviceHandle,
        deferred_operation: vk::DeferredOperationKHR,
        pipeline_cache: vk::PipelineCache,
        ray_tracing_pipeline_create_infos: &[vk::RayTracingPipelineCreateInfoKHR],
    ) -> Result<Vec<Self>> {
        // create ray_tracing pipeline
        let ray_tracing_pipelines = unsafe {
            device
                .ray_tracing_pipeline_loader_raw()
                .create_ray_tracing_pipelines(
                    deferred_operation,
                    pipeline_cache,
                    ray_tracing_pipeline_create_infos,
                    None,
                )
                .expect("Failed to create ray_tracing pipeline.")
        };

        let ray_tracing_pipelines = ray_tracing_pipelines
            .into_iter()
            .map(|ray_tracing_pipeline| Self {
                device: device.clone(),
                ray_tracing_pipeline,
                ref_count: AtomicUsize::new(1),
            })
            .collect::<Vec<_>>();

        Ok(ray_tracing_pipelines)
    }
}

/// vk::Pipelineを参照カウントで管理するためのハンドル
pub struct RayTracingPipelineHandle {
    ptr: NonNull<RayTracingPipelineHandleData>,
}
impl RayTracingPipelineHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        deferred_operation: vk::DeferredOperationKHR,
        pipeline_cache: vk::PipelineCache,
        ray_tracing_pipeline_create_infos: &[vk::RayTracingPipelineCreateInfoKHR],
    ) -> Vec<Self> {
        let data = Box::new(
            RayTracingPipelineHandleData::new(
                device_handle,
                deferred_operation,
                pipeline_cache,
                ray_tracing_pipeline_create_infos,
            )
            .expect("Failed to create ray_tracing pipeline."),
        );

        let ptrs = data
            .into_iter()
            .map(|data| unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) })
            .collect::<Vec<_>>();

        ptrs.into_iter().map(|ptr| Self { ptr }).collect()
    }

    // RayTracingPipelineの関数

    /// RayTracingShaderGroupHandlesを取得する
    pub fn get_ray_tracing_shader_group_handles(
        &self,
        first_group: u32,
        group_count: u32,
        data_size: usize,
    ) -> Vec<u8> {
        unsafe {
            self.data()
                .device
                .ray_tracing_pipeline_loader_raw()
                .get_ray_tracing_shader_group_handles(**self, first_group, group_count, data_size)
                .expect("Failed to get ray tracing shader group handles.")
        }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::Pipelineを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::Pipelineは無効になる。
    pub unsafe fn ray_tracing_pipeline_raw(&self) -> vk::Pipeline {
        self.data().ray_tracing_pipeline.clone()
    }

    fn data(&self) -> &RayTracingPipelineHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for RayTracingPipelineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RayTracingPipelineHandle").finish()
    }
}

// RayTracingPipelineHandleDataの中身はSendかつSyncなのでRayTracingPipelineHandleはSend
unsafe impl Send for RayTracingPipelineHandle {}
// RayTracingPipelineHandleDataの中身はSendかつSyncなのでRayTracingPipelineHandleはSync
unsafe impl Sync for RayTracingPipelineHandle {}

// RayTracingPipelineHandleはvk::PipelineにDerefする
impl Deref for RayTracingPipelineHandle {
    type Target = vk::Pipeline;
    fn deref(&self) -> &Self::Target {
        &self.data().ray_tracing_pipeline
    }
}

// Cloneで参照カウントを増やす
impl Clone for RayTracingPipelineHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to RayTracingPipelineHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for RayTracingPipelineHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // ray_tracing pipelineの破棄
                data.device
                    .destroy_pipeline(data.ray_tracing_pipeline, None);
            }
        }
    }
}
