//! 参照カウンタで管理して、参照がすべて破棄された際に
//! ShaderModuleの破棄の処理まで行うShaderModuleHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ShaderModuleHandleData {
    device: crate::DeviceHandle,
    shader_module: vk::ShaderModule,
    ref_count: AtomicUsize,
}
impl ShaderModuleHandleData {
    fn new(
        device: crate::DeviceHandle,
        shader_module_create_info: &vk::ShaderModuleCreateInfo,
    ) -> Result<Self> {
        // create shader module
        let shader_module =
            unsafe { ash::Device::create_shader_module(&device, shader_module_create_info, None)? };

        Ok(Self {
            device,
            shader_module,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::ShaderModuleを参照カウントで管理するためのハンドル
pub struct ShaderModuleHandle {
    ptr: NonNull<ShaderModuleHandleData>,
}
impl ShaderModuleHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        shader_module_create_info: &vk::ShaderModuleCreateInfo,
    ) -> Self {
        let data = Box::new(
            ShaderModuleHandleData::new(device_handle, shader_module_create_info)
                .expect("Failed to create shader module."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::ShaderModuleを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::ShaderModuleは無効になる。
    pub unsafe fn shader_module_raw(&self) -> vk::ShaderModule {
        self.data().shader_module.clone()
    }

    fn data(&self) -> &ShaderModuleHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for ShaderModuleHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShaderModuleHandle").finish()
    }
}

// ShaderModuleHandleDataの中身はSendかつSyncなのでShaderModuleHandleはSend
unsafe impl Send for ShaderModuleHandle {}
// ShaderModuleHandleDataの中身はSendかつSyncなのでShaderModuleHandleはSync
unsafe impl Sync for ShaderModuleHandle {}

// ShaderModuleHandleはvk::ShaderModuleにDerefする
impl Deref for ShaderModuleHandle {
    type Target = vk::ShaderModule;
    fn deref(&self) -> &Self::Target {
        &self.data().shader_module
    }
}

// Cloneで参照カウントを増やす
impl Clone for ShaderModuleHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to ShaderModuleHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for ShaderModuleHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // shader moduleの破棄
                data.device.destroy_shader_module(data.shader_module, None);
            }
        }
    }
}
