//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Bufferの破棄の処理まで行うBufferHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct BufferHandleData {
    device: crate::DeviceHandle,
    buffer: vk::Buffer,
    ref_count: AtomicUsize,
}
impl BufferHandleData {
    fn new(device: crate::DeviceHandle, buffer_create_info: &vk::BufferCreateInfo) -> Result<Self> {
        // create buffer
        let buffer = unsafe { ash::Device::create_buffer(&device, buffer_create_info, None)? };

        Ok(Self {
            device,
            buffer,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Bufferを参照カウントで管理するためのハンドル
pub struct BufferHandle {
    ptr: NonNull<BufferHandleData>,
}
impl BufferHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        buffer_create_info: &vk::BufferCreateInfo,
    ) -> Self {
        let data = BufferHandleData::new(device_handle, buffer_create_info)
            .expect("Failed to create buffer.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // bufferの各関数

    pub fn get_buffer_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.data()
                .device
                .get_buffer_memory_requirements(self.buffer_raw())
        }
    }

    pub fn bind_buffer_memory(&self, device_memory: vk::DeviceMemory, offset: u64) {
        unsafe {
            self.data()
                .device
                .bind_buffer_memory(self.buffer_raw(), device_memory, offset)
                .expect("Failed to bind buffer memory.");
        }
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn buffer_raw(&self) -> vk::Buffer {
        self.data().buffer.clone()
    }

    fn data(&self) -> &BufferHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for BufferHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferHandle").finish()
    }
}

// BufferHandleDataの中身はSendかつSyncなのでBufferHandleはSend
unsafe impl Send for BufferHandle {}
// BufferHandleDataの中身はSendかつSyncなのでBufferHandleはSync
unsafe impl Sync for BufferHandle {}

// BufferHandleはvk::BufferにDerefする
impl Deref for BufferHandle {
    type Target = vk::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.data().buffer
    }
}

// Cloneで参照カウントを増やす
impl Clone for BufferHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to BufferHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for BufferHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // bufferの破棄
                data.device.destroy_buffer(data.buffer, None);
            }
        }
    }
}
