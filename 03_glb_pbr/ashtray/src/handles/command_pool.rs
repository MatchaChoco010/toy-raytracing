//! 参照カウンタで管理して、参照がすべて破棄された際に
//! CommandPoolの破棄の処理まで行うCommandPoolHandleを定義する。

use anyhow::Result;
use ash::vk;
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct CommandPoolHandleData {
    device: crate::DeviceHandle,
    command_pool: vk::CommandPool,
    ref_count: AtomicUsize,
}
impl CommandPoolHandleData {
    fn new(
        device: crate::DeviceHandle,
        command_pool_create_info: &vk::CommandPoolCreateInfo,
    ) -> Result<Self> {
        // create command pool
        let command_pool =
            unsafe { ash::Device::create_command_pool(&device, command_pool_create_info, None)? };

        Ok(Self {
            device,
            command_pool,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::CommandPoolを参照カウントで管理するためのハンドル
pub struct CommandPoolHandle {
    ptr: NonNull<CommandPoolHandleData>,
}
impl CommandPoolHandle {
    pub(crate) fn new(
        device_handle: crate::DeviceHandle,
        command_pool_create_info: &vk::CommandPoolCreateInfo,
    ) -> Self {
        let data = Box::new(
            CommandPoolHandleData::new(device_handle, command_pool_create_info)
                .expect("Failed to create command pool."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // raw

    /// DeviceHandleを取得する
    pub fn device(&self) -> crate::DeviceHandle {
        self.data().device.clone()
    }

    /// vk::CommandPoolを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したvk::CommandPoolは無効になる。
    pub unsafe fn command_pool_raw(&self) -> vk::CommandPool {
        self.data().command_pool.clone()
    }

    fn data(&self) -> &CommandPoolHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for CommandPoolHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandPoolHandle").finish()
    }
}

// CommandPoolHandleDataの中身はSendかつSyncなのでCommandPoolHandleはSend
unsafe impl Send for CommandPoolHandle {}
// CommandPoolHandleDataの中身はSendかつSyncなのでCommandPoolHandleはSync
unsafe impl Sync for CommandPoolHandle {}

// CommandPoolHandleはvk::CommandPoolにDerefする
impl Deref for CommandPoolHandle {
    type Target = vk::CommandPool;
    fn deref(&self) -> &Self::Target {
        &self.data().command_pool
    }
}

// Cloneで参照カウントを増やす
impl Clone for CommandPoolHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to CommandPoolHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for CommandPoolHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // command poolの破棄
                data.device.destroy_command_pool(data.command_pool, None);
            }
        }
    }
}
