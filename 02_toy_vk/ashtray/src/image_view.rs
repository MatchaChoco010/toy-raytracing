//! 参照カウンタで管理して、参照がすべて破棄された際に
//! ImageViewの破棄の処理まで行うImageViewHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ImageViewHandleData {
    device: crate::DeviceHandle,
    image: crate::ImageHandle,
    image_view: vk::ImageView,
    ref_count: AtomicUsize,
}
impl ImageViewHandleData {
    fn new(
        device: crate::DeviceHandle,
        image: crate::ImageHandle,
        image_view_create_info: &vk::ImageViewCreateInfo,
    ) -> Result<Self> {
        // create image
        let image_view =
            unsafe { ash::Device::create_image_view(&device, image_view_create_info, None)? };

        Ok(Self {
            device,
            image,
            image_view,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Imageを参照カウントで管理するためのハンドル
pub struct ImageViewHandle {
    ptr: NonNull<ImageViewHandleData>,
}
impl ImageViewHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        image_handle: crate::ImageHandle,
        image_view_create_info: &vk::ImageViewCreateInfo,
    ) -> Self {
        let data = ImageViewHandleData::new(device_handle, image_handle, image_view_create_info)
            .expect("Failed to create image view.");
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    // image viewの各関数

    // raw

    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    pub fn image(&self) -> crate::ImageHandle {
        self.data().image.clone()
    }

    pub unsafe fn image_view_raw(&self) -> vk::ImageView {
        self.data().image_view.clone()
    }

    fn data(&self) -> &ImageViewHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for ImageViewHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageViewHandle").finish()
    }
}

// ImageViewHandleDataの中身はSendかつSyncなのでImageViewHandleはSend
unsafe impl Send for ImageViewHandle {}
// ImageViewHandleDataの中身はSendかつSyncなのでImageViewHandleはSync
unsafe impl Sync for ImageViewHandle {}

// ImageViewHandleはvk::ImageViewにDerefする
impl Deref for ImageViewHandle {
    type Target = vk::ImageView;
    fn deref(&self) -> &Self::Target {
        &self.data().image_view
    }
}

// Cloneで参照カウントを増やす
impl Clone for ImageViewHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to ImageViewHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for ImageViewHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // imageの破棄
                data.device.destroy_image_view(data.image_view, None);
            }
        }
    }
}
