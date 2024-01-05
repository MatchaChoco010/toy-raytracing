//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Allocationの破棄の処理まで行うAllocationHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct AllocationHandleData {
    device: crate::DeviceHandle,
    allocator: crate::AllocatorHandle,
    allocation: gpu_allocator::vulkan::Allocation,
    ref_count: AtomicUsize,
}
impl AllocationHandleData {
    fn new(
        device: crate::DeviceHandle,
        allocator: crate::AllocatorHandle,
        allocation_create_desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> Result<Self> {
        // create device memory
        let allocation = allocator
            .lock()
            .unwrap()
            .allocate(allocation_create_desc)
            .expect("Failed to allocate memory.");

        Ok(Self {
            device,
            allocator,
            allocation,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Imageを参照カウントで管理するためのハンドル
pub struct AllocationHandle {
    ptr: NonNull<AllocationHandleData>,
}
impl AllocationHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        allocator_handle: crate::AllocatorHandle,
        allocation_create_desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> Self {
        let data =
            AllocationHandleData::new(device_handle, allocator_handle, allocation_create_desc)
                .expect("Failed to allocate allocation.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // Allocationの関数

    /// vk::DeviceMemoryを取得する
    pub fn memory(&self) -> vk::DeviceMemory {
        unsafe { self.data().allocation.memory() }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// AllocatorHandleを取得する
    pub fn allocator(&self) -> crate::AllocatorHandle {
        self.data().allocator.clone()
    }

    fn data(&self) -> &AllocationHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for AllocationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllocationHandle").finish()
    }
}

// AllocationHandleDataの中身はSendかつSyncなのでAllocationHandleはSend
unsafe impl Send for AllocationHandle {}
// AllocationHandleDataの中身はSendかつSyncなのでAllocationHandleはSync
unsafe impl Sync for AllocationHandle {}

// AllocationHandleはvk::ImageViewにDerefする
impl Deref for AllocationHandle {
    type Target = gpu_allocator::vulkan::Allocation;
    fn deref(&self) -> &Self::Target {
        &self.data().allocation
    }
}

// Cloneで参照カウントを増やす
impl Clone for AllocationHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to AllocationHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for AllocationHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // device_memoryの破棄
                let mut allocator = data.allocator.lock().unwrap();
                allocator
                    .free(data.allocation)
                    .expect("Failed to free memory.")
            }
        }
    }
}
