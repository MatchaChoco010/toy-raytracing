//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Instanceの破棄の処理まで行うInstanceHandleを定義する。

use anyhow::Result;
use ash::{extensions::ext::DebugUtils, vk};
use std::{
    ffi::CStr,
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

#[cfg(feature = "validation")]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(feature = "validation"))]
const ENABLE_VALIDATION_LAYERS: bool = false;

const VALIDATION: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

// debug utilsのコールバック関数
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
        _ => panic!("[UNKNOWN]"),
    };
    let types = match message_types {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        _ => panic!("[UNKNOWN]"),
    };
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    println!("[DEBUG]{}{}{:?}", severity, types, message);

    vk::FALSE
}

pub struct InstanceHandleData {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_loader: DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    ref_count: AtomicUsize,
}
impl InstanceHandleData {
    fn new(raw_display_handle: raw_window_handle::RawDisplayHandle) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        // instanceの作成とdebug utilsの設定
        let (instance, debug_utils_loader, debug_messenger) = {
            let app_name = std::ffi::CString::new("Hello Triangle")?;
            let app_info = vk::ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(vk::make_api_version(1, 0, 0, 0))
                .api_version(vk::API_VERSION_1_3);
            let mut debug_utils_messenger_create_info =
                vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .flags(vk::DebugUtilsMessengerCreateFlagsEXT::empty())
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                    )
                    .pfn_user_callback(Some(vulkan_debug_utils_callback))
                    .build();
            let mut extension_names = vec![DebugUtils::name().as_ptr()];
            for &extension in ash_window::enumerate_required_extensions(raw_display_handle)? {
                let name = unsafe { CStr::from_ptr(extension).as_ptr() };
                extension_names.push(name);
            }
            let raw_layer_names = VALIDATION
                .iter()
                .map(|l| std::ffi::CString::new(*l).unwrap())
                .collect::<Vec<_>>();
            let layer_names = raw_layer_names
                .iter()
                .map(|l| l.as_ptr())
                .collect::<Vec<*const i8>>();
            let instance_create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(&extension_names);
            let instance_create_info = if ENABLE_VALIDATION_LAYERS {
                instance_create_info
                    .push_next(&mut debug_utils_messenger_create_info)
                    .enabled_layer_names(&layer_names)
            } else {
                instance_create_info
            };
            let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

            // setup debug utils
            let debug_utils_loader = DebugUtils::new(&entry, &instance);
            let debug_messenger = if ENABLE_VALIDATION_LAYERS {
                unsafe {
                    debug_utils_loader
                        .create_debug_utils_messenger(&debug_utils_messenger_create_info, None)?
                }
            } else {
                vk::DebugUtilsMessengerEXT::null()
            };

            (instance, debug_utils_loader, debug_messenger)
        };

        Ok(Self {
            entry,
            instance,
            debug_utils_loader,
            debug_messenger,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// vk::Instanceを参照カウントで管理するためのハンドル
pub struct InstanceHandle {
    ptr: NonNull<InstanceHandleData>,
}
impl InstanceHandle {
    pub fn new(raw_display_handle: raw_window_handle::RawDisplayHandle) -> Self {
        let data = Box::new(
            InstanceHandleData::new(raw_display_handle).expect("Failed to create instance."),
        );
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Self { ptr }
    }

    // create系

    pub fn create_surface(
        &self,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> crate::SurfaceHandle {
        crate::SurfaceHandle::new(self.clone(), raw_display_handle, raw_window_handle)
            .expect("Failed to create surface.")
    }

    pub fn create_device(
        &self,
        physical_device: vk::PhysicalDevice,
        device_create_info: &vk::DeviceCreateInfo,
    ) -> crate::DeviceHandle {
        crate::DeviceHandle::new(self.clone(), physical_device, device_create_info)
            .expect("Failed to create device.")
    }

    // instanceの各関数

    pub fn enumerate_physical_devices(&self) -> Vec<vk::PhysicalDevice> {
        unsafe {
            self.data()
                .instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices.")
        }
    }

    pub fn get_physical_device_queue_family_properties(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.data()
                .instance
                .get_physical_device_queue_family_properties(physical_device)
        }
    }

    pub fn get_physical_device_memory_properties(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.data()
                .instance
                .get_physical_device_memory_properties(physical_device)
        }
    }

    pub fn get_physical_device_properties2(
        &self,
        physical_device: vk::PhysicalDevice,
        props: &mut vk::PhysicalDeviceProperties2,
    ) {
        unsafe {
            self.data()
                .instance
                .get_physical_device_properties2(physical_device, props)
        }
    }

    // raw

    pub fn entry(&self) -> &ash::Entry {
        &self.data().entry
    }

    pub unsafe fn entry_raw(&self) -> ash::Entry {
        self.data().entry.clone()
    }

    pub unsafe fn instance_raw(&self) -> ash::Instance {
        self.data().instance.clone()
    }

    fn data(&self) -> &InstanceHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for InstanceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceHandle").finish()
    }
}

// InstanceHandleDataの中身はSendかつSyncなのでInstanceHandleはSend
unsafe impl Send for InstanceHandle {}
// InstanceHandleDataの中身はSendかつSyncなのでInstanceHandleはSync
unsafe impl Sync for InstanceHandle {}

// InstanceHandleはvk::InstanceにDerefする
impl Deref for InstanceHandle {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.data().instance
    }
}

// Cloneで参照カウントを増やす
impl Clone for InstanceHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to InstanceHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for InstanceHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // debug utilsの破棄
                if ENABLE_VALIDATION_LAYERS {
                    data.debug_utils_loader
                        .destroy_debug_utils_messenger(data.debug_messenger, None);
                }

                // instanceの破棄
                data.instance.destroy_instance(None);
            }
        }
    }
}
