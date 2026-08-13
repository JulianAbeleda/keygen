//! Generic native application host. Product crates provide only state and drawing.
use keygen_engine::{Canvas, Surface};
use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HostEvent {
    Key {
        key: Key,
        pressed: bool,
    },
    Text(char),
    Pointer {
        x: f32,
        y: f32,
        button: Option<MouseButton>,
        pressed: bool,
    },
    Tick(Duration),
    Close,
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
            app.event(HostEvent::Key { key, pressed: true });
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
            });
        }
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp) {
            let down = window.get_mouse_down(MouseButton::Left);
            let pointer = (x, y, down);
            if previous_pointer != Some(pointer) {
                app.event(HostEvent::Pointer {
                    x,
                    y,
                    button: Some(MouseButton::Left),
                    pressed: down,
                });
                previous_pointer = Some(pointer);
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
            window
                .update_with_buffer(
                    &canvas.surface().packed_rgb(),
                    physical_width,
                    physical_height,
                )
                .map_err(|e| e.to_string())?;
        } else {
            window.update();
        }
        frame += 1;
    }
    app.event(HostEvent::Close);
    Ok(())
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
    let mut canvas = Canvas::new(width as u32, height as u32, [0, 0, 0, 255]);
    app.frame(
        &mut canvas,
        HostContext {
            elapsed,
            frame,
            width,
            height,
        },
    );
    canvas.into_surface()
}
