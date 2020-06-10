use std::ffi::CStr;
use std::fmt;
use std::os::raw::{c_char, c_float, c_int, c_void};

use crate::cerror::CError;

extern "C" {
    fn XOpenDisplay(name: *const c_char) -> *mut c_void;
    fn XCloseDisplay(disp: *mut c_void) -> c_int;

    fn create_capture_infos(
        disp: *mut c_void,
        handles: *mut *mut c_void,
        size: c_int,
        err: *mut CError,
    ) -> c_int;

    fn clone_capture_info(handle: *const c_void) -> *mut c_void;
    fn destroy_capture_info(handle: *mut c_void);
    fn get_capture_name(handle: *const c_void) -> *const c_char;
    fn capture_before_input(handle: *mut c_void, err: *mut CError);
    fn get_geometry_relative(
        handle: *const c_void,
        x: *mut c_float,
        y: *mut c_float,
        width: *mut c_float,
        height: *mut c_float,
        err: *mut CError,
    );
}

pub struct Capture {
    handle: *mut c_void,
}

impl Clone for Capture {
    fn clone(&self) -> Self {
        let handle = unsafe { clone_capture_info(self.handle) };
        Self { handle }
    }
}

unsafe impl Send for Capture {}

impl Capture {
    pub unsafe fn handle(&mut self) -> *mut c_void {
        self.handle
    }

    pub fn name(&self) -> String {
        unsafe {
            CStr::from_ptr(get_capture_name(self.handle))
                .to_string_lossy()
                .into()
        }
    }

    pub fn geometry(&self) -> Result<CaptureGeometry, CError> {
        let mut x: c_float = 0.0;
        let mut y: c_float = 0.0;
        let mut width: c_float = 0.0;
        let mut height: c_float = 0.0;
        let mut err = CError::new();
        fltk::app::lock().unwrap();
        unsafe {
            get_geometry_relative(
                self.handle,
                &mut x,
                &mut y,
                &mut width,
                &mut height,
                &mut err,
            );
        }
        fltk::app::unlock();
        if err.is_err() {
            return Err(err);
        }
        Ok(CaptureGeometry {
            x: x.into(),
            y: y.into(),
            width: width.into(),
            height: height.into(),
        })
    }

    pub fn before_input(&mut self) -> Result<(), CError> {
        let mut err = CError::new();
        fltk::app::lock().unwrap();
        unsafe { capture_before_input(self.handle, &mut err) };
        fltk::app::unlock();
        if err.is_err() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for Capture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            destroy_capture_info(self.handle);
        }
    }
}

pub struct CaptureGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct X11Context {
    disp: *mut c_void,
}

impl X11Context {
    pub fn new() -> Option<Self> {
        let disp = unsafe { XOpenDisplay(std::ptr::null()) };
        if disp.is_null() {
            return None;
        }
        Some(Self { disp })
    }

    pub fn captures(&mut self) -> Result<Vec<Capture>, CError> {
        let mut err = CError::new();
        let mut handles = [std::ptr::null_mut::<c_void>(); 128];
        fltk::app::lock().unwrap();
        let size = unsafe {
            create_capture_infos(
                self.disp,
                handles.as_mut_ptr(),
                handles.len() as c_int,
                &mut err,
            )
        };
        fltk::app::unlock();
        if err.is_err() {
            return Err(err);
        }
        Ok(handles[0..size as usize]
            .iter()
            .map(|handle| Capture { handle: *handle })
            .collect::<Vec<Capture>>())
    }
}

impl Drop for X11Context {
    fn drop(&mut self) {
        fltk::app::lock().unwrap();
        unsafe { XCloseDisplay(self.disp) };
        fltk::app::unlock();
    }
}
