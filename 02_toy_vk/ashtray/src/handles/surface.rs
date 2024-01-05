//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Surfaceの破棄の処理まで行うSurfaceHandleを定義する。

use anyhow::Result;
use ash::{extensions::khr::Surface, vk};
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

pub struct SurfaceHandleData {
    instance: crate::InstanceHandle,
    surface_loader: Surface,
    surface: vk::SurfaceKHR,
    ref_count: AtomicUsize,
}
impl SurfaceHandleData {
    fn new(
        instance_handle: crate::InstanceHandle,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<Self> {
        // surfaceの作成
        let (surface_loader, surface) = unsafe {
            let surface_loader =
                ash::extensions::khr::Surface::new(&instance_handle.entry_raw(), &instance_handle);
            let surface = ash_window::create_surface(
                &instance_handle.entry_raw(),
                &instance_handle,
                raw_display_handle,
                raw_window_handle,
                None,
            )?;
            (surface_loader, surface)
        };

        Ok(Self {
            instance: instance_handle,
            surface_loader,
            surface,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::SurfaceKHRを参照カウントで管理するためのハンドル
pub struct SurfaceHandle {
    ptr: NonNull<SurfaceHandleData>,
}
impl SurfaceHandle {
    pub(crate) fn new(
        instance_handle: crate::InstanceHandle,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<Self> {
        let data = Box::new(SurfaceHandleData::new(
            instance_handle,
            raw_display_handle,
            raw_window_handle,
        )?);
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Ok(Self { ptr })
    }

    // surfaceの関数

    /// PhysicalDeviceがSurfaceをサポートしているか確認する
    pub fn get_physical_device_surface_support(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> bool {
        unsafe {
            self.data()
                .surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    queue_family_index,
                    self.data().surface,
                )
                .expect("Failed to get physical device surface support.")
        }
    }

    /// PhysicalDeviceのSurfaceのCapabilitiesを取得する
    pub fn get_physical_device_surface_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::SurfaceCapabilitiesKHR {
        unsafe {
            self.data()
                .surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.data().surface)
                .expect("Failed to get physical device surface capabilities.")
        }
    }

    /// PhysicalDeviceのSurfaceのFormatsを取得する
    pub fn get_physical_device_surface_formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Vec<vk::SurfaceFormatKHR> {
        unsafe {
            self.data()
                .surface_loader
                .get_physical_device_surface_formats(physical_device, self.data().surface)
                .expect("Failed to get physical device surface formats.")
        }
    }

    /// PhysicalDeviceのSurfaceのPresentModesを取得する
    pub fn get_physical_device_surface_present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Vec<vk::PresentModeKHR> {
        unsafe {
            self.data()
                .surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.data().surface)
                .expect("Failed to get physical device surface present modes.")
        }
    }

    // raw

    /// InstanceHandleを取得する
    pub fn instance(&self) -> crate::InstanceHandle {
        self.data().instance.clone()
    }

    /// SurfaceLoaderを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したSurfaceLoaderは無効になる。
    pub unsafe fn surface_loader_raw(&self) -> Surface {
        self.data().surface_loader.clone()
    }

    fn data(&self) -> &SurfaceHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for SurfaceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfaceHandle").finish()
    }
}

// SurfaceHandleDataの中身はSendかつSyncなのでSurfaceHandleはSend
unsafe impl Send for SurfaceHandle {}
// SurfaceHandleDataの中身はSendかつSyncなのでSurfaceHandleはSync
unsafe impl Sync for SurfaceHandle {}

// SurfaceHandleはvk::SurfaceにDerefする
impl Deref for SurfaceHandle {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.data().surface
    }
}

// Cloneで参照カウントを増やす
impl Clone for SurfaceHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to SurfaceHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for SurfaceHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // Surfaceの破棄
                data.surface_loader.destroy_surface(data.surface, None);
            }
        }
    }
}
