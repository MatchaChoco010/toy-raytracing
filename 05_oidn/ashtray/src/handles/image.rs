//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Imageの破棄の処理まで行うImageHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ImageHandleData {
    device: crate::DeviceHandle,
    image: vk::Image,
    ref_count: AtomicUsize,
}
impl ImageHandleData {
    fn new(device: crate::DeviceHandle, image_create_info: &vk::ImageCreateInfo) -> Result<Self> {
        // create image
        let image = unsafe { ash::Device::create_image(&device, image_create_info, None)? };

        Ok(Self {
            device,
            image,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Imageを参照カウントで管理するためのハンドル
pub struct ImageHandle {
    ptr: NonNull<ImageHandleData>,
}
impl ImageHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        image_create_info: &vk::ImageCreateInfo,
    ) -> Self {
        let data = ImageHandleData::new(device_handle, image_create_info)
            .expect("Failed to create image.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // imageの関数

    // その他のimageの関数

    /// Imageのメモリ要件を取得する
    pub fn get_image_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.data()
                .device
                .get_image_memory_requirements(self.image_raw())
        }
    }

    /// Imageのメモリバインドを行う
    pub fn bind_image_memory(&self, device_memory: vk::DeviceMemory, offset: u64) {
        unsafe {
            self.data()
                .device
                .bind_image_memory(self.image_raw(), device_memory, offset)
                .expect("Failed to bind image memory.");
        }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::Imageを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::Imageは無効になる。
    pub unsafe fn image_raw(&self) -> vk::Image {
        self.data().image.clone()
    }

    fn data(&self) -> &ImageHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for ImageHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageHandle").finish()
    }
}

// ImageHandleDataの中身はSendかつSyncなのでImageHandleはSend
unsafe impl Send for ImageHandle {}
// ImageHandleDataの中身はSendかつSyncなのでImageHandleはSync
unsafe impl Sync for ImageHandle {}

// ImageHandleはvk::ImageにDerefする
impl Deref for ImageHandle {
    type Target = vk::Image;
    fn deref(&self) -> &Self::Target {
        &self.data().image
    }
}

// Cloneで参照カウントを増やす
impl Clone for ImageHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to ImageHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for ImageHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // imageの破棄
                data.device.destroy_image(data.image, None);
            }
        }
    }
}
