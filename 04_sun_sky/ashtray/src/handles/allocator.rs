//! gpu-allocatorのAllocatorを保持するハンドルを定義する。
//! AllocatorはDrop時に自動で破棄される。

use anyhow::Result;
use gpu_allocator::vulkan::*;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::{Arc, Mutex},
};

struct AllocatorHandleData {
    device: crate::DeviceHandle,
    allocator: Arc<Mutex<Allocator>>,
}
impl AllocatorHandleData {
    fn new(
        device: crate::DeviceHandle,
        allocator_create_desc: &AllocatorCreateDesc,
    ) -> Result<Self> {
        // create allocator
        let allocator = Allocator::new(allocator_create_desc)?;
        let allocator = Arc::new(Mutex::new(allocator));

        Ok(Self { device, allocator })
    }
}

/// gpu_allocatorを参照カウントで管理するためのハンドル
#[derive(Clone)]
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

    // Allocatorの関数

    /// AllocationHandleを割り当てる
    pub fn allocate(
        &self,
        allocation_create_desc: &gpu_allocator::vulkan::AllocationCreateDesc,
    ) -> crate::AllocationHandle {
        crate::AllocationHandle::new(self.device(), self.clone(), allocation_create_desc)
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// Allocatorを取得する
    pub fn allocator(&self) -> Arc<Mutex<Allocator>> {
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
