use oidn_sys::*;
use std::{
    fmt::Debug,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct OidnDeviceData {
    device: OIDNDevice,
    ref_count: AtomicUsize,
}
impl OidnDeviceData {
    pub fn new() -> Self {
        let device = unsafe { oidnNewDevice(OIDNDeviceType::OIDN_DEVICE_TYPE_DEFAULT) };
        unsafe { oidnCommitDevice(device) };

        unsafe {
            let mut error = std::ptr::null();
            oidnGetDeviceError(device, &mut error);
            if !error.is_null() {
                let error = std::ffi::CStr::from_ptr(error);
                panic!("OIDN new device error: {:?}", error);
            }
        }

        Self {
            device,
            ref_count: AtomicUsize::new(1),
        }
    }
}

pub struct OidnDevice {
    ptr: NonNull<OidnDeviceData>,
}
impl OidnDevice {
    pub fn new() -> Self {
        let data = OidnDeviceData::new();
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    pub fn new_filter(&self, ty: impl Into<String>) -> crate::OidnFilter {
        crate::OidnFilter::new(self, ty)
    }

    pub fn new_buffer(&self, buffer: &ashtray::utils::SharedBuffer) -> crate::OidnBuffer {
        crate::OidnBuffer::new(self, buffer)
    }

    pub(crate) fn device_raw(&self) -> OIDNDevice {
        self.data().device
    }

    fn data(&self) -> &OidnDeviceData {
        unsafe { self.ptr.as_ref() }
    }
}

impl Debug for OidnDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidnDevice").finish()
    }
}

impl Clone for OidnDevice {
    fn clone(&self) -> Self {
        self.data().ref_count.fetch_add(1, Ordering::SeqCst);
        Self { ptr: self.ptr }
    }
}

impl Drop for OidnDevice {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                oidnReleaseDevice(data.device);
            }
        }
    }
}
