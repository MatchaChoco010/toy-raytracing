//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Samplerの破棄の処理まで行うSamplerHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct SamplerHandleData {
    device: crate::DeviceHandle,
    sampler: vk::Sampler,
    ref_count: AtomicUsize,
}
impl SamplerHandleData {
    fn new(
        device: crate::DeviceHandle,
        sampler_create_info: &vk::SamplerCreateInfo,
    ) -> Result<Self> {
        // create sampler
        let sampler = unsafe { ash::Device::create_sampler(&device, sampler_create_info, None)? };

        Ok(Self {
            device,
            sampler,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Samplerを参照カウントで管理するためのハンドル
pub struct SamplerHandle {
    ptr: NonNull<SamplerHandleData>,
}
impl SamplerHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        sampler_create_info: &vk::SamplerCreateInfo,
    ) -> Self {
        let data = SamplerHandleData::new(device_handle, sampler_create_info)
            .expect("Failed to create sampler.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::Samplerを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::Samplerは無効になる。
    pub unsafe fn sampler_raw(&self) -> vk::Sampler {
        self.data().sampler.clone()
    }

    fn data(&self) -> &SamplerHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for SamplerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SamplerHandle").finish()
    }
}

// SamplerHandleDataの中身はSendかつSyncなのでSamplerHandleはSend
unsafe impl Send for SamplerHandle {}
// SamplerHandleDataの中身はSendかつSyncなのでSamplerHandleはSync
unsafe impl Sync for SamplerHandle {}

// SamplerHandleはvk::ImageにDerefする
impl Deref for SamplerHandle {
    type Target = vk::Sampler;
    fn deref(&self) -> &Self::Target {
        &self.data().sampler
    }
}

// Cloneで参照カウントを増やす
impl Clone for SamplerHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to SamplerHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for SamplerHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // samplerの破棄
                data.device.destroy_sampler(data.sampler, None);
            }
        }
    }
}
