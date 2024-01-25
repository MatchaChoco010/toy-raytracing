use oidn_sys::*;
use std::{
    fmt::Debug,
    ptr::null,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

use crate::OidnDevice;

struct OidnBufferData {
    _device: OidnDevice,
    buffer: OIDNBuffer,
    ref_count: AtomicUsize,
}
impl OidnBufferData {
    fn new(device: &OidnDevice, buffer: &ashtray::utils::SharedBuffer) -> Self {
        #[cfg(target_os = "windows")]
        let buffer = unsafe {
            let name = null();
            let handle = buffer.handle;
            oidnNewSharedBufferFromWin32Handle(
                device.device_raw(),
                OIDNExternalMemoryTypeFlag::OIDN_EXTERNAL_MEMORY_TYPE_FLAG_OPAQUE_WIN32,
                handle,
                name,
                buffer.size as usize,
            )
        };
        #[cfg(target_os = "linux")]
        let buffer = unsafe {
            let fd = buffer.fd;
            oidnNewSharedBufferFromFD(
                device.device_raw(),
                OIDNExternalMemoryTypeFlag::OIDN_EXTERNAL_MEMORY_TYPE_FLAG_OPAQUE_FD,
                fd,
                buffer.size as usize,
            )
        };

        unsafe {
            let mut error = std::ptr::null();
            oidnGetDeviceError(device.device_raw(), &mut error);
            if !error.is_null() {
                let error = std::ffi::CStr::from_ptr(error);
                panic!("OIDN new buffer error: {:?}", error);
            }
        }

        Self {
            _device: device.clone(),
            buffer,
            ref_count: AtomicUsize::new(1),
        }
    }
}

pub struct OidnBuffer {
    ptr: NonNull<OidnBufferData>,
}
impl OidnBuffer {
    pub(crate) fn new(device: &OidnDevice, buffer: &ashtray::utils::SharedBuffer) -> Self {
        let data = OidnBufferData::new(device, buffer);
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    pub fn buffer_raw(&self) -> OIDNBuffer {
        self.data().buffer
    }

    fn data(&self) -> &OidnBufferData {
        unsafe { self.ptr.as_ref() }
    }
}

impl Debug for OidnBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidnBuffer").finish()
    }
}

impl Clone for OidnBuffer {
    fn clone(&self) -> Self {
        self.data().ref_count.fetch_add(1, Ordering::SeqCst);
        Self { ptr: self.ptr }
    }
}

impl Drop for OidnBuffer {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                oidnReleaseBuffer(data.buffer);
            }
        }
    }
}
