use oidn_sys::*;
use std::{
    ffi::CString,
    fmt::Debug,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

use crate::OidnDevice;

struct OidnFilterData {
    _device: OidnDevice,
    filter: OIDNFilter,
    width: u32,
    height: u32,
    ref_count: AtomicUsize,
}
impl OidnFilterData {
    pub fn new(device: &OidnDevice, ty: String) -> Self {
        let ty = CString::new(ty).unwrap();
        let filter = unsafe { oidnNewFilter(device.device_raw(), ty.as_ptr()) };

        unsafe {
            let mut error = std::ptr::null();
            oidnGetDeviceError(device.device_raw(), &mut error);
            if !error.is_null() {
                let error = std::ffi::CStr::from_ptr(error);
                panic!("OIDN new filter error: {:?}", error);
            }
        }

        Self {
            _device: device.clone(),
            filter,
            width: 400,
            height: 300,
            ref_count: AtomicUsize::new(1),
        }
    }
}

pub struct OidnFilter {
    ptr: NonNull<OidnFilterData>,
}
impl OidnFilter {
    pub(crate) fn new(device: &OidnDevice, ty: impl Into<String>) -> Self {
        let data = OidnFilterData::new(device, ty.into());
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(data))) };
        Self { ptr }
    }

    pub fn hdr(&self, flag: bool) {
        let name = CString::new("hdr").unwrap();
        unsafe { oidnSetFilterBool(self.filter_raw(), name.as_ptr(), flag) };
    }

    pub fn srgb(&self, flag: bool) {
        let name = CString::new("srgb").unwrap();
        unsafe { oidnSetFilterBool(self.filter_raw(), name.as_ptr(), flag) };
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.data_mut().width = width;
        self.data_mut().height = height;
    }

    pub fn color(&self, buffer: &crate::OidnBuffer) {
        let name = CString::new("color").unwrap();
        unsafe {
            oidnSetFilterImage(
                self.data().filter,
                name.as_ptr(),
                buffer.buffer_raw(),
                OIDNFormat::OIDN_FORMAT_FLOAT3,
                self.data().width as usize,
                self.data().height as usize,
                0,
                0,
                0,
            )
        };
    }

    pub fn albedo(&self, buffer: &crate::OidnBuffer) {
        let name = CString::new("albedo").unwrap();
        unsafe {
            oidnSetFilterImage(
                self.data().filter,
                name.as_ptr(),
                buffer.buffer_raw(),
                OIDNFormat::OIDN_FORMAT_FLOAT3,
                self.data().width as usize,
                self.data().height as usize,
                0,
                0,
                0,
            )
        };
    }

    pub fn normal(&self, buffer: &crate::OidnBuffer) {
        let name = CString::new("normal").unwrap();
        unsafe {
            oidnSetFilterImage(
                self.data().filter,
                name.as_ptr(),
                buffer.buffer_raw(),
                OIDNFormat::OIDN_FORMAT_FLOAT3,
                self.data().width as usize,
                self.data().height as usize,
                0,
                0,
                0,
            )
        };
    }

    pub fn output(&self, buffer: &crate::OidnBuffer) {
        let name = CString::new("output").unwrap();
        unsafe {
            oidnSetFilterImage(
                self.data().filter,
                name.as_ptr(),
                buffer.buffer_raw(),
                OIDNFormat::OIDN_FORMAT_FLOAT3,
                self.data().width as usize,
                self.data().height as usize,
                0,
                0,
                0,
            )
        };
    }

    pub fn execute(&self) {
        unsafe { oidnCommitFilter(self.filter_raw()) };
        unsafe { oidnExecuteFilter(self.filter_raw()) };
        unsafe {
            let mut error = std::ptr::null();
            oidnGetDeviceError(self.data()._device.device_raw(), &mut error);
            if !error.is_null() {
                let error = std::ffi::CStr::from_ptr(error);
                panic!("OIDN error: {:?}", error);
            }
        }
    }

    pub(crate) fn filter_raw(&self) -> OIDNFilter {
        self.data().filter
    }

    fn data(&self) -> &OidnFilterData {
        unsafe { self.ptr.as_ref() }
    }

    fn data_mut(&mut self) -> &mut OidnFilterData {
        unsafe { self.ptr.as_mut() }
    }
}

impl Debug for OidnFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidnFilter").finish()
    }
}

impl Clone for OidnFilter {
    fn clone(&self) -> Self {
        self.data().ref_count.fetch_add(1, Ordering::SeqCst);
        Self { ptr: self.ptr }
    }
}

impl Drop for OidnFilter {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());
                oidnReleaseFilter(data.filter);
            }
        }
    }
}
