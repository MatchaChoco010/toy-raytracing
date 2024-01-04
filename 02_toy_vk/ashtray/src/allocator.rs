//! 参照カウンタで管理して、参照がすべて破棄された際に
//! gpu allocatorの破棄の処理まで行うAllocatorHandleを定義する。

use anyhow::Result;
use gpu_allocator::vulkan::*;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::{
        atomic::{fence, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

struct AllocatorHandleData {
    device: crate::DeviceHandle,
    allocator: Arc<Mutex<Allocator>>,
    ref_count: AtomicUsize,
}
impl AllocatorHandleData {
    fn new(
        device: crate::DeviceHandle,
        allocator_create_desc: &AllocatorCreateDesc,
    ) -> Result<Self> {
        // create allocator
        let allocator = Allocator::new(allocator_create_desc)?;
        let allocator = Arc::new(Mutex::new(allocator));

        Ok(Self {
            device,
            allocator,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// gpu_allocatorを参照カウントで管理するためのハンドル
pub struct AllocatorHandle {
    ptr: NonNull<AllocatorHandleData>,
}
impl AllocatorHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        allocator_create_desc: &AllocatorCreateDesc,
    ) -> Self {
        let data = Box::new(
            AllocatorHandleData::new(device_handle, allocator_create_desc)
                .expect("Failed to create allocator."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // allocatorの各関数

    pub fn allocate(
        &self,
        allocation_create_desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> crate::AllocationHandle {
        crate::AllocationHandle::new(self.device(), self.clone(), allocation_create_desc)
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn allocator_raw(&self) -> Arc<Mutex<Allocator>> {
        self.data().allocator.clone()
    }

    fn data(&self) -> &AllocatorHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for AllocatorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllocatorHandle").finish()
    }
}

// AllocatorHandleDataの中身はSendかつSyncなのでAllocatorHandleはSend
unsafe impl Send for AllocatorHandle {}
// AllocatorHandleDataの中身はSendかつSyncなのでAllocatorHandleはSync
unsafe impl Sync for AllocatorHandle {}

// AllocatorHandleはvk::CommandPoolにDerefする
impl Deref for AllocatorHandle {
    type Target = Arc<Mutex<Allocator>>;
    fn deref(&self) -> &Self::Target {
        &self.data().allocator
    }
}

// Cloneで参照カウントを増やす
impl Clone for AllocatorHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to AllocatorHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for AllocatorHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // gpu allocatorの破棄
                drop(data.allocator)
            }
        }
    }
}
