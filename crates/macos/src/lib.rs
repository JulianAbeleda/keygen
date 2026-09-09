//! Safe API over the small AppKit boundary required by KeyGen's native host.

const MAX_WINDOW_DIMENSION: usize = 4096;

/// Bounded, borrowed RGBA input for the host-only compositor. Coordinates are
/// physical pixels; the caller retains its deterministic CPU fallback.
pub struct RasterOverlay<'a> {
    pub pixels: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub rect: [f32; 4],
    pub brightness: f32,
    pub opacity: f32,
}

fn rgba_len(width: usize, height: usize) -> Result<usize, String> {
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err("compositor dimensions must be 1..=8192".into());
    }
    Ok(width * height * 4)
}

fn validate_overlay(
    base: &[u8],
    width: usize,
    height: usize,
    overlay: &RasterOverlay<'_>,
) -> Result<(), String> {
    if base.len() != rgba_len(width, height)?
        || overlay.pixels.len() != rgba_len(overlay.width, overlay.height)?
    {
        return Err("compositor RGBA length differs from dimensions".into());
    }
    if overlay
        .rect
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 32768.0)
        || overlay.rect[2] <= 0.0
        || overlay.rect[3] <= 0.0
        || !overlay.brightness.is_finite()
        || !(0.0..=16.0).contains(&overlay.brightness)
        || !overlay.opacity.is_finite()
        || !(0.0..=1.0).contains(&overlay.opacity)
    {
        return Err("compositor transform is invalid".into());
    }
    Ok(())
}

/// Optional synchronous Metal compute adapter, owned by a native host. No
/// platform requirement leaks into headless rendering. Results become visible
/// only after successful completion; failure leaves caller input untouched.
pub struct RasterCompositor {
    #[cfg(target_os = "macos")]
    inner: appkit::MetalCompositor,
}

impl RasterCompositor {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                inner: appkit::MetalCompositor::new()?,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("Metal composition is unavailable on this host".into())
        }
    }

    pub fn compose(
        &mut self,
        base: &[u8],
        width: usize,
        height: usize,
        overlay: RasterOverlay<'_>,
    ) -> Result<Vec<u8>, String> {
        validate_overlay(base, width, height, &overlay)?;
        #[cfg(target_os = "macos")]
        {
            self.inner.compose(base, width, height, &overlay)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err("Metal composition is unavailable on this host".into())
        }
    }
}

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

    // All Objective-C ABI calls and raw buffer access stay in this boundary.
    // Owned objects release once, borrowed inputs live through synchronous GPU
    // completion, and &mut self forbids concurrent buffer reuse. Raw object
    // pointers make the adapter !Send/!Sync; none escape its safe interface.
    macro_rules! message {
        ($object:expr, $selector:literal, ($($kind:ty),*) -> $result:ty $(, $arg:expr)*) => {{
            let call: unsafe extern "C" fn(*mut c_void, *mut c_void $(, $kind)*) -> $result =
                std::mem::transmute(objc_msgSend as *const ());
            call($object, selector($selector) $(, $arg)*)
        }};
    }

    struct Object(*mut c_void);
    impl Object {
        fn owned(value: *mut c_void, name: &str) -> Result<Self, String> {
            if value.is_null() {
                Err(format!("Metal could not create {name}"))
            } else {
                Ok(Self(value))
            }
        }
    }
    impl Drop for Object {
        fn drop(&mut self) {
            unsafe {
                message!(self.0, b"release\0", () -> ());
            }
        }
    }
    struct Pool(*mut c_void);
    impl Pool {
        fn new() -> Self {
            Self(unsafe { objc_autoreleasePoolPush() })
        }
    }
    impl Drop for Pool {
        fn drop(&mut self) {
            unsafe {
                objc_autoreleasePoolPop(self.0);
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Grid {
        width: usize,
        height: usize,
        depth: usize,
    }

    pub(super) struct MetalCompositor {
        device: Object,
        queue: Object,
        pipeline: Object,
        buffers: Option<([Object; 3], [usize; 2])>,
    }

    impl MetalCompositor {
        pub(super) fn new() -> Result<Self, String> {
            let _pool = Pool::new();
            unsafe {
                let device = Object::owned(MTLCreateSystemDefaultDevice(), "device")?;
                let queue = Object::owned(
                    message!(device.0, b"newCommandQueue\0", () -> *mut c_void),
                    "queue",
                )?;
                let options = Object::owned(
                    message!(objc_getClass(c"MTLCompileOptions".as_ptr()), b"new\0", () -> *mut c_void),
                    "compile options",
                )?;
                // Legacy selector supports older macOS too and maps to safe /
                // precise math. The shader also explicitly forbids contraction.
                message!(options.0, b"setFastMathEnabled:\0", (i8) -> (), 0);
                let source = string(METAL_COMPOSITION)?;
                let library = Object::owned(
                    message!(device.0, b"newLibraryWithSource:options:error:\0",
                    (*mut c_void, *mut c_void, *mut *mut c_void) -> *mut c_void,
                    source.0, options.0, std::ptr::null_mut()),
                    "library",
                )?;
                let name = string("compose")?;
                let function = Object::owned(
                    message!(library.0, b"newFunctionWithName:\0", (*mut c_void) -> *mut c_void, name.0),
                    "function",
                )?;
                let pipeline = Object::owned(
                    message!(device.0, b"newComputePipelineStateWithFunction:error:\0",
                    (*mut c_void, *mut *mut c_void) -> *mut c_void, function.0, std::ptr::null_mut()),
                    "pipeline",
                )?;
                Ok(Self {
                    device,
                    queue,
                    pipeline,
                    buffers: None,
                })
            }
        }

        pub(super) fn compose(
            &mut self,
            base: &[u8],
            width: usize,
            height: usize,
            overlay: &super::RasterOverlay<'_>,
        ) -> Result<Vec<u8>, String> {
            let _pool = Pool::new();
            let lengths = [base.len(), overlay.pixels.len()];
            unsafe {
                if self.buffers.as_ref().is_none_or(|(_, old)| old != &lengths) {
                    // One exact-sized set, replaced on geometry change; no
                    // unbounded texture cache. No command remains in flight.
                    let buffer = |length| {
                        Object::owned(
                            message!(self.device.0, b"newBufferWithLength:options:\0", (usize, usize) -> *mut c_void, length, 0),
                            "shared buffer",
                        )
                    };
                    self.buffers = Some((
                        [
                            buffer(lengths[0])?,
                            buffer(lengths[1])?,
                            buffer(lengths[0])?,
                        ],
                        lengths,
                    ));
                }
                let (buffers, _) = self.buffers.as_ref().expect("buffers allocated");
                for (buffer, bytes) in buffers.iter().zip([base, overlay.pixels]) {
                    let destination = message!(buffer.0, b"contents\0", () -> *mut u8);
                    if destination.is_null() {
                        return Err("Metal shared buffer has no contents".into());
                    }
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
                }
                let command = message!(self.queue.0, b"commandBuffer\0", () -> *mut c_void);
                if command.is_null() {
                    return Err("Metal command buffer unavailable".into());
                }
                let encoder = message!(command, b"computeCommandEncoder\0", () -> *mut c_void);
                if encoder.is_null() {
                    return Err("Metal compute encoder unavailable".into());
                }
                message!(encoder, b"setComputePipelineState:\0", (*mut c_void) -> (), self.pipeline.0);
                for (index, buffer) in buffers.iter().enumerate() {
                    message!(encoder, b"setBuffer:offset:atIndex:\0", (*mut c_void, usize, usize) -> (), buffer.0, 0, index);
                }
                let params = [
                    width as f32,
                    height as f32,
                    overlay.width as f32,
                    overlay.height as f32,
                    overlay.rect[0],
                    overlay.rect[1],
                    overlay.rect[2],
                    overlay.rect[3],
                    overlay.brightness,
                    overlay.opacity,
                ];
                message!(encoder, b"setBytes:length:atIndex:\0", (*const f32, usize, usize) -> (), params.as_ptr(), std::mem::size_of_val(&params), 3);
                message!(encoder, b"dispatchThreads:threadsPerThreadgroup:\0", (Grid, Grid) -> (),
                    Grid { width, height, depth: 1 }, Grid { width: 16, height: 16, depth: 1 });
                message!(encoder, b"endEncoding\0", () -> ());
                message!(command, b"commit\0", () -> ());
                message!(command, b"waitUntilCompleted\0", () -> ());
                if message!(command, b"status\0", () -> usize) != 4 {
                    return Err("Metal composition command failed".into());
                }
                let bytes = message!(buffers[2].0, b"contents\0", () -> *const u8);
                if bytes.is_null() {
                    return Err("Metal output has no contents".into());
                }
                Ok(std::slice::from_raw_parts(bytes, lengths[0]).to_vec())
            }
        }
    }

    unsafe fn string(value: &str) -> Result<Object, String> {
        let value = std::ffi::CString::new(value).map_err(|e| e.to_string())?;
        let allocated =
            message!(objc_getClass(c"NSString".as_ptr()), b"alloc\0", () -> *mut c_void);
        Object::owned(
            message!(allocated, b"initWithUTF8String:\0", (*const i8) -> *mut c_void, value.as_ptr()),
            "string",
        )
    }

    // Original, CPU-matched NP-010 scaled-overlay kernel. Byte-domain arithmetic
    // deliberately keeps brightness truncation and bilinear rounding distinct.
    const METAL_COMPOSITION: &str = r#"
    #include <metal_stdlib>
    #pragma clang fp contract(off)
    using namespace metal;
    float4 bright(device const uchar4 *s, uint i, float b) {
        float4 c = float4(s[i]);
        if (abs(b-1.0f) >= 1.1920929e-7f) c.rgb = floor(min(c.rgb*b, float3(255.0f)));
        return c;
    }
    kernel void compose(device const uchar4 *base [[buffer(0)]],
                        device const uchar4 *screen [[buffer(1)]],
                        device uchar4 *out [[buffer(2)]],
                        constant float *p [[buffer(3)]], uint2 xy [[thread_position_in_grid]]) {
        uint w=uint(p[0]), h=uint(p[1]), sw=uint(p[2]), sh=uint(p[3]);
        if (xy.x>=w || xy.y>=h) return;
        uint i=xy.y*w+xy.x;
        uchar4 dst=base[i]; out[i]=dst;
        float left=p[4], top=p[5], dw=p[6], dh=p[7], b=p[8], opacity=p[9];
        if (dw<=0 || dh<=0 || opacity<=0) return;
        float tw=max(floor(dw+0.5f),1.0f), th=max(floor(dh+0.5f),1.0f);
        if (float(xy.x)<max(floor(left),0.0f) || float(xy.y)<max(floor(top),0.0f) ||
            float(xy.x)>=min(ceil(left+tw),float(w)) || float(xy.y)>=min(ceil(top+th),float(h))) return;
        float px=clamp((float(xy.x)+0.5f-left)*float(sw)/dw-0.5f,0.0f,float(sw-1));
        float py=clamp((float(xy.y)+0.5f-top)*float(sh)/dh-0.5f,0.0f,float(sh-1));
        uint x0=uint(floor(px)), y0=uint(floor(py)), x1=min(x0+1,sw-1), y1=min(y0+1,sh-1);
        float fx=px-float(x0), fy=py-float(y0);
        float4 color=float4(0);
        color += bright(screen,y0*sw+x0,b)*((1.0f-fx)*(1.0f-fy));
        color += bright(screen,y0*sw+x1,b)*(fx*(1.0f-fy));
        color += bright(screen,y1*sw+x0,b)*((1.0f-fx)*fy);
        color += bright(screen,y1*sw+x1,b)*(fx*fy);
        color=floor(color+0.5f);
        float sa=clamp(color.a/255.0f*clamp(opacity,0.0f,1.0f),0.0f,1.0f);
        float da=float(dst.a)/255.0f, oa=sa+da*(1.0f-sa);
        float3 rgb=color.rgb*sa+float3(dst.rgb)*da*(1.0f-sa);
        out[i]=uchar4(uchar3(floor((oa>0 ? rgb/oa : float3(0))+0.5f)),uchar(floor(oa*255.0f+0.5f)));
    }
    "#;

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

    #[link(name = "Metal", kind = "framework")]
    unsafe extern "C" {
        fn MTLCreateSystemDefaultDevice() -> *mut c_void;
    }

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const i8) -> *mut c_void;
        fn sel_registerName(name: *const i8) -> *mut c_void;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);
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
    fn raster_inputs_fail_closed_without_a_gpu() {
        let bytes = [0u8; 16];
        let mut overlay = RasterOverlay {
            pixels: &bytes,
            width: 2,
            height: 2,
            rect: [-1.25, 0.5, 3.0, 2.0],
            brightness: 1.5,
            opacity: 0.5,
        };
        assert!(validate_overlay(&bytes, 2, 2, &overlay).is_ok());
        assert!(validate_overlay(&bytes[..15], 2, 2, &overlay).is_err());
        assert!(validate_overlay(&bytes, 0, 2, &overlay).is_err());
        assert!(validate_overlay(&bytes, usize::MAX, 2, &overlay).is_err());
        for value in [f32::NAN, f32::INFINITY, -1.0, 17.0] {
            overlay.brightness = value;
            assert!(validate_overlay(&bytes, 2, 2, &overlay).is_err());
        }
        overlay.brightness = 1.0;
        overlay.rect[0] = f32::NAN;
        assert!(validate_overlay(&bytes, 2, 2, &overlay).is_err());
        overlay.rect[0] = 0.0;
        overlay.opacity = f32::NAN;
        assert!(validate_overlay(&bytes, 2, 2, &overlay).is_err());
    }

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
