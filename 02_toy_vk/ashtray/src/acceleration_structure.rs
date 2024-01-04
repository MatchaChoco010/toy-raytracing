//! 参照カウンタで管理して、参照がすべて破棄された際に
//! AccelerationStructureの破棄の処理まで行うAccelerationStructureHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct AccelerationStructureHandleData {
    device: crate::DeviceHandle,
    acceleration_structure: vk::AccelerationStructureKHR,
    ref_count: AtomicUsize,
}
impl AccelerationStructureHandleData {
    fn new(
        device: crate::DeviceHandle,
        acceleration_structure_create_info: &vk::AccelerationStructureCreateInfoKHR,
    ) -> Result<Self> {
        // create acceleration_structure
        let acceleration_structure = unsafe {
            device
                .acceleration_structure_loader_raw()
                .create_acceleration_structure(acceleration_structure_create_info, None)?
        };

        Ok(Self {
            device,
            acceleration_structure,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::AccelerationStructureを参照カウントで管理するためのハンドル
pub struct AccelerationStructureHandle {
    ptr: NonNull<AccelerationStructureHandleData>,
}
impl AccelerationStructureHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        acceleration_structure_create_info: &vk::AccelerationStructureCreateInfoKHR,
    ) -> Self {
        let data =
            AccelerationStructureHandleData::new(device_handle, acceleration_structure_create_info)
                .expect("Failed to create acceleration_structure.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // 関数

    pub fn get_acceleration_structure_device_address(&self) -> u64 {
        unsafe {
            self.data()
                .device
                .acceleration_structure_loader_raw()
                .get_acceleration_structure_device_address(
                    &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                        .acceleration_structure(**self),
                )
        }
    }

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub unsafe fn acceleration_structure_raw(&self) -> vk::AccelerationStructureKHR {
        self.data().acceleration_structure.clone()
    }

    fn data(&self) -> &AccelerationStructureHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for AccelerationStructureHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccelerationStructureHandle").finish()
    }
}

// AccelerationStructureHandleDataの中身はSendかつSyncなのでAccelerationStructureHandleはSend
unsafe impl Send for AccelerationStructureHandle {}
// AccelerationStructureHandleDataの中身はSendかつSyncなのでAccelerationStructureHandleはSync
unsafe impl Sync for AccelerationStructureHandle {}

// AccelerationStructureHandleはvk::AccelerationStructureにDerefする
impl Deref for AccelerationStructureHandle {
    type Target = vk::AccelerationStructureKHR;
    fn deref(&self) -> &Self::Target {
        &self.data().acceleration_structure
    }
}

// Cloneで参照カウントを増やす
impl Clone for AccelerationStructureHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to AccelerationStructureHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for AccelerationStructureHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // acceleration_structureの破棄
                data.device
                    .acceleration_structure_loader_raw()
                    .destroy_acceleration_structure(data.acceleration_structure, None);
            }
        }
    }
}
