//! Safe API over the small AppKit boundary required by KeyGen's native host.

const MAX_WINDOW_DIMENSION: usize = 4096;

/// Measured native outer-frame geometry in global top-left logical points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
}

fn validate_dimensions(width: usize, height: usize) -> Result<(), String> {
    if width == 0 || height == 0 || width > MAX_WINDOW_DIMENSION || height > MAX_WINDOW_DIMENSION {
        return Err("window dimensions must be between 1 and 4096 points".into());
    }
    Ok(())
}

fn top_left_frame(
    x: f64,
    bottom: f64,
    width: f64,
    height: f64,
    primary_top: f64,
) -> Result<WindowFrame, String> {
    let values = [x, bottom, width, height, primary_top];
    if values.iter().any(|value| !value.is_finite()) || width <= 0.0 || height <= 0.0 {
        return Err("native window returned invalid frame geometry".into());
    }
    let x = x.round();
    let y = (primary_top - bottom - height).round();
    let width = width.round();
    let height = height.round();
    if x < f64::from(i32::MIN)
        || x > f64::from(i32::MAX)
        || y < f64::from(i32::MIN)
        || y > f64::from(i32::MAX)
        || width < 1.0
        || width > MAX_WINDOW_DIMENSION as f64
        || height < 1.0
        || height > MAX_WINDOW_DIMENSION as f64
    {
        return Err("native window frame is outside supported bounds".into());
    }
    Ok(WindowFrame {
        x: x as i32,
        y: y as i32,
        width: width as usize,
        height: height as usize,
    })
}

#[cfg(target_os = "macos")]
mod appkit {
    use core::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSRect {
        origin: NSPoint,
        size: NSSize,
    }

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const i8) -> *mut c_void;
        fn sel_registerName(name: *const i8) -> *mut c_void;
        fn objc_msgSend();
    }

    pub(super) fn set_size(width: usize, height: usize) -> Result<(), String> {
        unsafe {
            let window = application_window()?;
            let current = object_rect(window, selector(b"frame\0"));
            let frame = NSRect {
                origin: NSPoint {
                    x: current.origin.x,
                    y: current.origin.y + current.size.height - height as f64,
                },
                size: NSSize {
                    width: width as f64,
                    height: height as f64,
                },
            };
            set_frame(window, frame);
        }
        Ok(())
    }

    pub(super) fn set_bounds(x: i32, y: i32, width: usize, height: usize) -> Result<(), String> {
        unsafe {
            let window = application_window()?;
            let primary_top = primary_top()?;
            set_frame(
                window,
                NSRect {
                    origin: NSPoint {
                        x: f64::from(x),
                        y: primary_top - f64::from(y) - height as f64,
                    },
                    size: NSSize {
                        width: width as f64,
                        height: height as f64,
                    },
                },
            );
        }
        Ok(())
    }

    pub(super) fn observe() -> Result<super::WindowFrame, String> {
        unsafe {
            let window = application_window()?;
            let frame = object_rect(window, selector(b"frame\0"));
            super::top_left_frame(
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
                primary_top()?,
            )
        }
    }

    unsafe fn set_frame(window: *mut c_void, frame: NSRect) {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void, NSRect, i8) =
            std::mem::transmute(objc_msgSend as *const ());
        send(window, selector(b"setFrame:display:\0"), frame, 1);
    }

    unsafe fn object_rect(object: *mut c_void, selector: *mut c_void) -> NSRect {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NSRect =
            std::mem::transmute(objc_msgSend as *const ());
        send(object, selector)
    }

    unsafe fn primary_top() -> Result<f64, String> {
        let frame = object_rect(primary_screen()?, selector(b"frame\0"));
        Ok(frame.origin.y + frame.size.height)
    }

    unsafe fn primary_screen() -> Result<*mut c_void, String> {
        let class = objc_getClass(c"NSScreen".as_ptr());
        let screens = send_object(class, selector(b"screens\0"));
        if screens.is_null() || send_count(screens, selector(b"count\0")) == 0 {
            Err("cannot locate the primary display".into())
        } else {
            Ok(send_index(screens, 0))
        }
    }

    unsafe fn selector(name: &'static [u8]) -> *mut c_void {
        sel_registerName(name.as_ptr().cast())
    }

    unsafe fn send_object(object: *mut c_void, selector: *mut c_void) -> *mut c_void {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        send(object, selector)
    }

    unsafe fn send_count(object: *mut c_void, selector: *mut c_void) -> usize {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
            std::mem::transmute(objc_msgSend as *const ());
        send(object, selector)
    }

    unsafe fn send_index(object: *mut c_void, index: usize) -> *mut c_void {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        send(object, selector(b"objectAtIndex:\0"), index)
    }

    unsafe fn application_window() -> Result<*mut c_void, String> {
        let class = objc_getClass(c"NSApplication".as_ptr());
        if class.is_null() {
            return Err("cannot locate NSApplication".into());
        }
        let application = send_object(class, selector(b"sharedApplication\0"));
        let mut window = send_object(application, selector(b"mainWindow\0"));
        if window.is_null() {
            window = send_object(application, selector(b"keyWindow\0"));
        }
        if window.is_null() {
            let windows = send_object(application, selector(b"windows\0"));
            if !windows.is_null() && send_count(windows, selector(b"count\0")) > 0 {
                window = send_index(windows, 0);
            }
        }
        if window.is_null() {
            Err("cannot locate the application's native window".into())
        } else {
            Ok(window)
        }
    }
}

pub fn set_window_size(width: usize, height: usize) -> Result<(), String> {
    validate_dimensions(width, height)?;
    #[cfg(target_os = "macos")]
    {
        appkit::set_size(width, height)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (width, height);
        Err("in-place native window sizing is unsupported on this host".into())
    }
}

pub fn set_window_bounds(x: i32, y: i32, width: usize, height: usize) -> Result<(), String> {
    validate_dimensions(width, height)?;
    if x == i32::MIN || y == i32::MIN {
        return Err("window coordinates are outside supported bounds".into());
    }
    #[cfg(target_os = "macos")]
    {
        appkit::set_bounds(x, y, width, height)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y, width, height);
        Err("in-place native window bounds are unsupported on this host".into())
    }
}

pub fn observe_window_frame() -> Result<WindowFrame, String> {
    #[cfg(target_os = "macos")]
    {
        appkit::observe()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("native window observation is unsupported on this host".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_left_conversion_preserves_negative_origins() {
        assert_eq!(
            top_left_frame(-1440.0, 120.0, 960.0, 628.0, 1117.0).unwrap(),
            WindowFrame {
                x: -1440,
                y: 369,
                width: 960,
                height: 628,
            }
        );
    }

    #[test]
    fn frame_and_request_bounds_fail_closed() {
        assert!(validate_dimensions(0, 628).is_err());
        assert!(validate_dimensions(960, 4097).is_err());
        assert!(set_window_size(0, 628).is_err());
        assert!(set_window_bounds(i32::MIN, 0, 960, 628).is_err());
        assert!(top_left_frame(0.0, 0.0, f64::NAN, 628.0, 1117.0).is_err());
        assert!(top_left_frame(0.0, 0.0, 960.0, -1.0, 1117.0).is_err());
    }
}
