//! 参照カウンタで管理して、参照がすべて破棄された際に
//! CommandBufferの破棄の処理まで行うCommandBufferHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct CommandBufferHandleData {
    device: crate::DeviceHandle,
    command_pool: crate::CommandPoolHandle,
    command_buffer: vk::CommandBuffer,
    ref_count: AtomicUsize,
}
impl CommandBufferHandleData {
    fn allocate(
        device: crate::DeviceHandle,
        command_pool: crate::CommandPoolHandle,
        command_buffer_allocate_info: &vk::CommandBufferAllocateInfo,
    ) -> Result<Vec<Self>> {
        // create command pool
        let command_buffer = unsafe {
            ash::Device::allocate_command_buffers(&device, command_buffer_allocate_info)?
        };

        let command_buffers = command_buffer
            .into_iter()
            .map(|command_buffer| Self {
                device: device.clone(),
                command_pool: command_pool.clone(),
                command_buffer,
                ref_count: AtomicUsize::new(1),
            })
            .collect();

        Ok(command_buffers)
    }
}

/// vk::CommandBufferを参照カウントで管理するためのハンドル
pub struct CommandBufferHandle {
    ptr: NonNull<CommandBufferHandleData>,
}
impl CommandBufferHandle {
    pub(crate) fn allocate(
        device_handle: crate::DeviceHandle,
        command_pool: crate::CommandPoolHandle,
        command_buffer_allocate_info: &vk::CommandBufferAllocateInfo,
    ) -> Vec<Self> {
        let data = CommandBufferHandleData::allocate(
            device_handle,
            command_pool,
            command_buffer_allocate_info,
        )
        .expect("Failed to allocate command buffers.");

        let ptrs = data
            .into_iter()
            .map(|data| unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) })
            .collect::<Vec<_>>();
        ptrs.into_iter().map(|ptr| Self { ptr }).collect()
    }

    // CommandBufferの関数

    /// CommandBufferを開始する
    pub fn begin_command_buffer(&self, begin_info: &vk::CommandBufferBeginInfo) {
        unsafe {
            self.data()
                .device
                .begin_command_buffer(self.command_buffer_raw(), begin_info)
                .expect("Failed to begin command buffer.")
        }
    }

    /// CommandBufferを終了する
    pub fn end_command_buffer(&self) {
        unsafe {
            self.data()
                .device
                .end_command_buffer(self.command_buffer_raw())
                .expect("Failed to end command buffer.")
        }
    }

    /// CommandBufferをリセットする
    pub fn reset_command_buffer(&self, flags: vk::CommandBufferResetFlags) {
        unsafe {
            self.data()
                .device
                .reset_command_buffer(self.command_buffer_raw(), flags)
                .expect("Failed to reset command buffer.")
        }
    }

    /// clear color imageコマンドを積む
    pub fn cmd_clear_color_image(
        &self,
        image: &vk::Image,
        image_layout: vk::ImageLayout,
        clear_color: &vk::ClearColorValue,
        ranges: &[vk::ImageSubresourceRange],
    ) {
        unsafe {
            self.data().device.cmd_clear_color_image(
                self.command_buffer_raw(),
                *image,
                image_layout,
                clear_color,
                ranges,
            )
        }
    }

    /// copy bufferコマンドを積む
    pub fn cmd_copy_buffer(
        &self,
        src_buffer: &crate::BufferHandle,
        dst_buffer: &crate::BufferHandle,
        regions: &[vk::BufferCopy],
    ) {
        unsafe {
            self.data().device.cmd_copy_buffer(
                self.command_buffer_raw(),
                **src_buffer,
                **dst_buffer,
                regions,
            )
        }
    }

    /// copy buffer to imageコマンドを積む
    pub fn cmd_copy_buffer_to_image(
        &self,
        src_buffer: &crate::BufferHandle,
        dst_image: &crate::ImageHandle,
        dst_image_layout: vk::ImageLayout,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.data().device.cmd_copy_buffer_to_image(
                self.command_buffer_raw(),
                **src_buffer,
                **dst_image,
                dst_image_layout,
                regions,
            )
        }
    }

    /// pipeline barrier2コマンドを積む
    pub fn cmd_pipeline_barrier2(&self, dependency_info: &vk::DependencyInfoKHR) {
        unsafe {
            self.data()
                .device
                .cmd_pipeline_barrier2(self.command_buffer_raw(), dependency_info);
        }
    }

    /// ComputePipelineHandleをbindするコマンドを積む
    pub fn cmd_bind_compute_pipeline(&self, compute_pipeline: &crate::ComputePipelineHandle) {
        unsafe {
            self.data().device.cmd_bind_pipeline(
                self.command_buffer_raw(),
                vk::PipelineBindPoint::COMPUTE,
                **compute_pipeline,
            )
        }
    }

    /// RayTracingPipelineHandleをbindするコマンドを積む
    pub fn cmd_bind_ray_tracing_pipeline(
        &self,
        ray_tracing_pipeline: &crate::RayTracingPipelineHandle,
    ) {
        unsafe {
            self.data().device.cmd_bind_pipeline(
                self.command_buffer_raw(),
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                **ray_tracing_pipeline,
            )
        }
    }

    /// DescriptorSetHandleをbindするコマンドを積む
    pub fn cmd_bind_descriptor_sets(
        &self,
        pipeline_bind_point: vk::PipelineBindPoint,
        pipeline_layout: &crate::PipelineLayoutHandle,
        first_set: u32,
        descriptor_sets: &[crate::DescriptorSetHandle],
        dynamic_offsets: &[u32],
    ) {
        unsafe {
            self.data().device.cmd_bind_descriptor_sets(
                self.command_buffer_raw(),
                pipeline_bind_point,
                **pipeline_layout,
                first_set,
                descriptor_sets
                    .iter()
                    .map(|descriptor_set| **descriptor_set)
                    .collect::<Vec<_>>()
                    .as_slice(),
                dynamic_offsets,
            )
        }
    }

    /// Dispatchコマンドを積む
    pub fn cmd_dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe {
            self.data()
                .device
                .cmd_dispatch(self.command_buffer_raw(), x, y, z)
        }
    }

    /// PushConstantsを積むコマンドを積む
    pub fn cmd_push_constants<T: bytemuck::Pod>(
        &self,
        pipeline_layout: &crate::PipelineLayoutHandle,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        values: &[T],
    ) {
        unsafe {
            self.data().device.cmd_push_constants(
                self.command_buffer_raw(),
                **pipeline_layout,
                stage_flags,
                offset,
                bytemuck::cast_slice(values),
            )
        }
    }

    /// acceleration structureを構築するコマンドを積む
    pub fn cmd_build_acceleration_structures(
        &self,
        build_infos: &[vk::AccelerationStructureBuildGeometryInfoKHR],
        build_range_infos: &[&[vk::AccelerationStructureBuildRangeInfoKHR]],
    ) {
        unsafe {
            self.data()
                .device
                .acceleration_structure_loader_raw()
                .cmd_build_acceleration_structures(
                    self.command_buffer_raw(),
                    build_infos,
                    build_range_infos,
                )
        }
    }

    /// RayTracingを起動するコマンドを積む
    pub fn cmd_trace_rays(
        &self,
        raygen_shader_binding_table_entry: &vk::StridedDeviceAddressRegionKHR,
        miss_shader_binding_table_entry: &vk::StridedDeviceAddressRegionKHR,
        hit_shader_binding_table_entry: &vk::StridedDeviceAddressRegionKHR,
        callable_shader_binding_table_entry: &vk::StridedDeviceAddressRegionKHR,
        width: u32,
        height: u32,
        depth: u32,
    ) {
        unsafe {
            self.data()
                .device
                .ray_tracing_pipeline_loader_raw()
                .cmd_trace_rays(
                    self.command_buffer_raw(),
                    raygen_shader_binding_table_entry,
                    miss_shader_binding_table_entry,
                    hit_shader_binding_table_entry,
                    callable_shader_binding_table_entry,
                    width,
                    height,
                    depth,
                )
        }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::CommandBufferを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::CommandBufferは無効になる。
    pub unsafe fn command_buffer_raw(&self) -> vk::CommandBuffer {
        self.data().command_buffer.clone()
    }

    fn data(&self) -> &CommandBufferHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイト実装
impl Debug for CommandBufferHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBufferHandle").finish()
    }
}

// CommandBufferHandleDataの中身はSendかつSyncなのでCommandBufferHandleはSend
unsafe impl Send for CommandBufferHandle {}
// CommandBufferHandleDataの中身はSendかつSyncなのでCommandBufferHandleはSync
unsafe impl Sync for CommandBufferHandle {}

// CommandBufferHandleはvk::CommandBufferにDerefする
impl Deref for CommandBufferHandle {
    type Target = vk::CommandBuffer;
    fn deref(&self) -> &Self::Target {
        &self.data().command_buffer
    }
}

// Cloneで参照カウントを増やす
impl Clone for CommandBufferHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to CommandBufferHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for CommandBufferHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // command bufferの破棄
                data.device
                    .free_command_buffers(*data.command_pool, &[data.command_buffer])
            }
        }
    }
}
