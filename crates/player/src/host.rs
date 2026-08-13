//! Generic native application host. Product crates provide only state and drawing.
use keygen_engine::{Canvas, Surface};
use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HostEvent {
    Key {
        key: Key,
        pressed: bool,
        modifiers: Modifiers,
    },
    Text(char),
    Pointer {
        x: f32,
        y: f32,
        button: Option<MouseButton>,
        pressed: bool,
    },
    Scroll {
        delta: i32,
    },
    Tick(Duration),
    Close,
}

/// Modifier state sampled with each key event. This is deliberately generic
/// host metadata; applications decide what combinations mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// Convert a native vertical wheel delta into a bounded, discrete event.
/// Invalid and zero values are ignored.
pub(crate) fn normalize_scroll_delta(value: f32) -> Option<i32> {
    if !value.is_finite() || value == 0.0 {
        return None;
    }
    let magnitude = value.abs().round().clamp(1.0, 32.0) as i32;
    Some(if value.is_sign_negative() {
        -magnitude
    } else {
        magnitude
    })
}

#[derive(Clone, Debug)]
pub struct HostContext {
    pub elapsed: Duration,
    pub frame: u64,
    pub width: usize,
    pub height: usize,
}

pub trait Application {
    fn frame(&mut self, canvas: &mut Canvas, context: HostContext);
    fn event(&mut self, _event: HostEvent) {}
    /// Return false when the previously presented buffer remains valid. The
    /// host will continue polling native events without rerasterizing it.
    fn needs_redraw(&self) -> bool {
        true
    }
    fn should_close(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub struct WindowPolicy {
    pub width: usize,
    pub height: usize,
    /// Physical raster pixels per logical drawing pixel. A value of 2 renders
    /// crisp text on common Retina displays while preserving app coordinates.
    pub pixel_density: u8,
    /// Maximum presentation rate. Keeping this explicit prevents static native
    /// screens from consuming a full CPU core in the platform event loop.
    pub target_fps: usize,
    pub resizable: bool,
    pub fullscreen: bool,
    pub title: String,
}

impl WindowPolicy {
    /// Return drawable backing dimensions for this logical window.
    pub fn physical_size(&self) -> Result<(usize, usize), String> {
        let density = usize::from(self.pixel_density);
        let width = self
            .width
            .checked_mul(density)
            .ok_or_else(|| "window backing width overflows usize".to_owned())?;
        let height = self
            .height
            .checked_mul(density)
            .ok_or_else(|| "window backing height overflows usize".to_owned())?;
        Ok((width, height))
    }
}

impl Default for WindowPolicy {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            pixel_density: 1,
            target_fps: 60,
            resizable: true,
            fullscreen: false,
            title: "KeyGen".into(),
        }
    }
}

/// Run an application in a native minifb window. The application owns all state;
/// this host only translates platform input and presents its deterministic frame.
pub fn run<A: Application>(mut app: A, policy: WindowPolicy) -> Result<(), String> {
    if !(1..=4).contains(&policy.pixel_density) {
        return Err("window pixel_density must be between 1 and 4".into());
    }
    if !(1..=240).contains(&policy.target_fps) {
        return Err("window target_fps must be between 1 and 240".into());
    }
    policy.physical_size()?;
    let mut options = WindowOptions {
        resize: policy.resizable,
        ..WindowOptions::default()
    };
    options.borderless = policy.fullscreen;
    let mut window = Window::new(&policy.title, policy.width, policy.height, options)
        .map_err(|e| e.to_string())?;
    window.set_target_fps(policy.target_fps);
    let text = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(TextCollector { text: text.clone() }));
    let started = Instant::now();
    let mut previous = started;
    let mut frame = 0;
    let mut previous_pointer = None;
    let mut previous_left_down = false;
    // minifb's macOS Metal backend consumes the submitted pixel pointer on an
    // asynchronous display callback. Keep several complete submissions alive
    // so neither a transition frame nor the final static frame can point at a
    // temporary Vec that Rust has already released.
    let mut presented_frames: VecDeque<Vec<u32>> = VecDeque::with_capacity(5);
    while window.is_open() && !app.should_close() {
        let now = Instant::now();
        let delta = now.duration_since(previous);
        previous = now;
        let (window_width, window_height) = window.get_size();
        let context = HostContext {
            elapsed: now.duration_since(started),
            frame,
            width: window_width,
            height: window_height,
        };
        app.event(HostEvent::Tick(delta));
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            app.event(HostEvent::Key {
                key,
                pressed: true,
                modifiers: modifiers(&window),
            });
        }
        for codepoint in text.borrow_mut().drain(..) {
            if let Some(character) = char::from_u32(codepoint) {
                app.event(HostEvent::Text(character));
            }
        }
        for key in window.get_keys_released() {
            app.event(HostEvent::Key {
                key,
                pressed: false,
                modifiers: modifiers(&window),
            });
        }
        if let Some((_, vertical)) = window.get_scroll_wheel() {
            if let Some(delta) = normalize_scroll_delta(vertical) {
                app.event(HostEvent::Scroll { delta });
            }
        }
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp) {
            let down = window.get_mouse_down(MouseButton::Left);
            let pointer = (x, y);
            if previous_pointer != Some(pointer) {
                app.event(HostEvent::Pointer {
                    x,
                    y,
                    button: None,
                    pressed: down,
                });
                previous_pointer = Some(pointer);
            }
            if down != previous_left_down {
                app.event(HostEvent::Pointer {
                    x,
                    y,
                    button: Some(MouseButton::Left),
                    pressed: down,
                });
                previous_left_down = down;
            }
        }
        if frame == 0 || app.needs_redraw() {
            let mut canvas = Canvas::new_scaled(
                window_width as u32,
                window_height as u32,
                f32::from(policy.pixel_density),
                [0, 0, 0, 255],
            );
            app.frame(&mut canvas, context);
            let physical_width = canvas.surface().width as usize;
            let physical_height = canvas.surface().height as usize;
            presented_frames.push_back(canvas.surface().packed_rgb());
            window
                .update_with_buffer(
                    presented_frames.back().expect("presented frame was queued"),
                    physical_width,
                    physical_height,
                )
                .map_err(|e| e.to_string())?;
            // Three Metal buffers may be in flight. Keep those plus the latest
            // submitted frame alive until a later presentation advances them.
            while presented_frames.len() > 4 {
                presented_frames.pop_front();
            }
        } else {
            window.update();
        }
        frame += 1;
    }
    app.event(HostEvent::Close);
    Ok(())
}

fn modifiers(window: &Window) -> Modifiers {
    Modifiers {
        shift: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
        ctrl: window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl),
        alt: window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt),
        super_key: window.is_key_down(Key::LeftSuper) || window.is_key_down(Key::RightSuper),
    }
}

struct TextCollector {
    text: Rc<RefCell<Vec<u32>>>,
}

impl InputCallback for TextCollector {
    fn add_char(&mut self, codepoint: u32) {
        self.text.borrow_mut().push(codepoint);
    }
}

/// Render one dynamic frame without opening a window, useful for tests and captures.
pub fn render_frame<A: Application>(
    app: &mut A,
    width: usize,
    height: usize,
    elapsed: Duration,
    frame: u64,
) -> Surface {
    render_frame_scaled(app, width, height, 1, frame, elapsed.as_millis() as u64)
        .expect("density 1 headless render cannot fail")
}

/// Render a logical frame into a physical-resolution backing surface without
/// opening a window. `view` is the deterministic frame/view identifier and
/// `now_ms` is supplied by the caller so captures do not read a clock.
pub fn render_frame_scaled<A: Application>(
    app: &mut A,
    logical_width: usize,
    logical_height: usize,
    pixel_density: u8,
    view: u64,
    now_ms: u64,
) -> Result<Surface, String> {
    if !(1..=4).contains(&pixel_density) {
        return Err("headless pixel_density must be between 1 and 4".into());
    }
    let density = usize::from(pixel_density);
    let width = logical_width
        .checked_mul(density)
        .ok_or_else(|| "headless backing width overflows usize".to_owned())?;
    let height = logical_height
        .checked_mul(density)
        .ok_or_else(|| "headless backing height overflows usize".to_owned())?;
    let width_u32 =
        u32::try_from(width).map_err(|_| "headless backing width exceeds u32".to_owned())?;
    let height_u32 =
        u32::try_from(height).map_err(|_| "headless backing height exceeds u32".to_owned())?;
    let logical_width_u32 =
        u32::try_from(logical_width).map_err(|_| "logical width exceeds u32".to_owned())?;
    let logical_height_u32 =
        u32::try_from(logical_height).map_err(|_| "logical height exceeds u32".to_owned())?;
    debug_assert_eq!(width_u32, logical_width_u32 * u32::from(pixel_density));
    debug_assert_eq!(height_u32, logical_height_u32 * u32::from(pixel_density));
    let mut canvas = Canvas::new_scaled(
        logical_width_u32,
        logical_height_u32,
        f32::from(pixel_density),
        [0, 0, 0, 255],
    );
    app.frame(
        &mut canvas,
        HostContext {
            elapsed: Duration::from_millis(now_ms),
            frame: view,
            width: logical_width,
            height: logical_height,
        },
    );
    Ok(canvas.into_surface())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SolidApp;
    impl Application for SolidApp {
        fn frame(&mut self, canvas: &mut Canvas, _context: HostContext) {
            canvas.clear([255, 0, 0, 255]);
        }
    }

    #[test]
    fn scaled_headless_render_uses_physical_dimensions() {
        let surface = render_frame_scaled(&mut SolidApp, 3, 2, 2, 7, 125).unwrap();
        assert_eq!((surface.width, surface.height), (6, 4));
    }

    #[test]
    fn scaled_headless_render_rejects_invalid_density() {
        assert!(render_frame_scaled(&mut SolidApp, 3, 2, 0, 0, 0).is_err());
        assert!(render_frame_scaled(&mut SolidApp, 3, 2, 5, 0, 0).is_err());
    }

    #[test]
    fn policy_maps_logical_window_to_backing_pixels() {
        let policy = WindowPolicy {
            width: 1600,
            height: 900,
            pixel_density: 2,
            ..WindowPolicy::default()
        };
        assert_eq!(policy.physical_size().unwrap(), (3200, 1800));
    }

    #[test]
    fn policy_rejects_backing_dimension_overflow() {
        let policy = WindowPolicy {
            width: usize::MAX,
            pixel_density: 2,
            ..WindowPolicy::default()
        };
        assert!(policy.physical_size().is_err());
    }

    #[test]
    fn scroll_normalization_discards_invalid_and_zero_values() {
        assert_eq!(normalize_scroll_delta(0.0), None);
        assert_eq!(normalize_scroll_delta(f32::NAN), None);
        assert_eq!(normalize_scroll_delta(f32::INFINITY), None);
        assert_eq!(normalize_scroll_delta(f32::NEG_INFINITY), None);
    }

    #[test]
    fn scroll_normalization_rounds_preserves_sign_and_clamps() {
        assert_eq!(normalize_scroll_delta(0.1), Some(1));
        assert_eq!(normalize_scroll_delta(-0.1), Some(-1));
        assert_eq!(normalize_scroll_delta(1.6), Some(2));
        assert_eq!(normalize_scroll_delta(-1.6), Some(-2));
        assert_eq!(normalize_scroll_delta(100.0), Some(32));
        assert_eq!(normalize_scroll_delta(-100.0), Some(-32));
    }

    #[test]
    fn key_event_carries_generic_modifiers() {
        let event = HostEvent::Key {
            key: Key::A,
            pressed: true,
            modifiers: Modifiers {
                shift: true,
                ctrl: false,
                alt: true,
                super_key: false,
            },
        };
        assert_eq!(
            event,
            HostEvent::Key {
                key: Key::A,
                pressed: true,
                modifiers: Modifiers {
                    shift: true,
                    ctrl: false,
                    alt: true,
                    super_key: false,
                },
            }
        );
        assert_eq!(
            HostEvent::Scroll { delta: -3 },
            HostEvent::Scroll { delta: -3 }
        );
    }
}
