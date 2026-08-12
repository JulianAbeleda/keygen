#![forbid(unsafe_code)]
//! Filesystem loading and native presentation for KeyGen scenes.

use keygen_engine::{model::SceneSpec, Scene, SceneAssets};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, ScaleMode, Window, WindowOptions};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Debug)]
struct Args {
    scene: PathBuf,
    render: Option<PathBuf>,
    time: f32,
    smoke_seconds: Option<f32>,
    validate: bool,
}

pub fn run_cli<I, S>(arguments: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    run(parse_args(arguments)?)
}

pub fn load_scene(path: &Path) -> Result<Scene, String> {
    let document =
        fs::read(path).map_err(|error| format!("cannot read scene {}: {error}", path.display()))?;
    let mut spec = SceneSpec::from_json(&document)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    resolve_scene_paths(&mut spec, base);

    let font = read_asset(&spec.font_path, "font")?;
    let mut layers = HashMap::new();
    for layer in &spec.layers {
        layers.insert(layer.id.clone(), read_asset(&layer.path, "layer")?);
    }
    let particle = spec
        .particles
        .as_ref()
        .map(|value| read_asset(&value.path, "particle"))
        .transpose()?;
    Scene::from_assets(
        spec,
        SceneAssets {
            font,
            layers,
            particle,
        },
    )
}

fn read_asset(path: &str, kind: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {kind} asset {path}: {error}"))
}

fn resolve_scene_paths(spec: &mut SceneSpec, base: &Path) {
    spec.font_path = resolve_path(base, &spec.font_path);
    for layer in &mut spec.layers {
        layer.path = resolve_path(base, &layer.path);
    }
    if let Some(particles) = &mut spec.particles {
        particles.path = resolve_path(base, &particles.path);
    }
}

fn resolve_path(base: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() {
        value.to_owned()
    } else {
        base.join(path).to_string_lossy().into_owned()
    }
}

pub fn write_png(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn parse_args<I, S>(arguments: I) -> Result<Args, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut values = arguments
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned());
    let _program = values.next();
    let mut scene = None;
    let mut render = None;
    let mut time = 4.25;
    let mut smoke_seconds = None;
    let mut validate = false;
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--scene" => scene = Some(PathBuf::from(values.next().ok_or("--scene needs a path")?)),
            "--render" => {
                render = Some(PathBuf::from(values.next().ok_or("--render needs a path")?))
            }
            "--time" => {
                time = values
                    .next()
                    .ok_or("--time needs seconds")?
                    .parse()
                    .map_err(|_| "invalid --time")?
            }
            "--smoke-seconds" => {
                smoke_seconds = Some(
                    values
                        .next()
                        .ok_or("--smoke-seconds needs seconds")?
                        .parse()
                        .map_err(|_| "invalid smoke duration")?,
                )
            }
            "--validate" => validate = true,
            "--help" | "-h" => {
                return Err(
                    "usage: keygen --scene FILE [--render PNG] [--time SECONDS] \
                     [--smoke-seconds SECONDS] [--validate]"
                        .into(),
                )
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(Args {
        scene: scene.ok_or("--scene is required")?,
        render,
        time,
        smoke_seconds,
        validate,
    })
}

fn run(args: Args) -> Result<(), String> {
    let scene = load_scene(&args.scene)?;
    if args.validate {
        println!(
            "scene OK: {} ({}x{}, {} layers, {} menu entries)",
            scene.spec.title,
            scene.spec.design_width,
            scene.spec.design_height,
            scene.spec.layers.len(),
            scene.spec.menu.entries.len()
        );
        return Ok(());
    }
    if let Some(output) = args.render {
        write_png(&output, &scene.render(args.time, 0).encode_png()?)?;
        println!("rendered {}", output.display());
        return Ok(());
    }
    run_window(scene, args.smoke_seconds)
}

fn run_window(scene: Scene, smoke_seconds: Option<f32>) -> Result<(), String> {
    let width = scene.spec.design_width as usize;
    let height = scene.spec.design_height as usize;
    let mut window = Window::new(
        &scene.spec.title,
        width,
        height,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .map_err(|error| format!("cannot create native window: {error}"))?;
    window.set_target_fps(60);
    let start = Instant::now();
    let mut focused = first_enabled(&scene, 0);
    let mut mouse_was_down = false;
    let mut pressed_entry = None;
    while window.is_open() {
        let elapsed = start.elapsed().as_secs_f32();
        if smoke_seconds.is_some_and(|seconds| elapsed >= seconds) {
            break;
        }
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            break;
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
            focused = next_enabled(&scene, focused, 1);
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
            focused = next_enabled(&scene, focused, -1);
        }
        let window_size = window.get_size();
        let hovered = if let Some((mouse_x, mouse_y)) = window.get_mouse_pos(MouseMode::Discard) {
            let (design_x, design_y) = map_pointer(mouse_x, mouse_y, window_size, (width, height));
            let hit = scene.menu_hit(design_x, design_y);
            if let Some(index) = hit {
                focused = index;
            }
            hit
        } else {
            None
        };
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if !mouse_was_down && mouse_down {
            pressed_entry = hovered;
        }
        let pointer_activated = mouse_was_down
            && !mouse_down
            && pressed_entry
                .take()
                .is_some_and(|entry| Some(entry) == hovered);
        let activated = window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::Space, KeyRepeat::No)
            || pointer_activated;
        mouse_was_down = mouse_down;
        if activated {
            let entry = &scene.spec.menu.entries[focused];
            println!("menu action: {}", entry.id);
            if entry.id == "exit" {
                break;
            }
        }
        let frame = scene.render(elapsed, focused).packed_rgb();
        window
            .update_with_buffer(&frame, width, height)
            .map_err(|error| format!("native frame failed: {error}"))?;
    }
    Ok(())
}

fn first_enabled(scene: &Scene, start: usize) -> usize {
    scene
        .spec
        .menu
        .entries
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, entry)| entry.enabled.then_some(index))
        .unwrap_or(0)
}

fn next_enabled(scene: &Scene, current: usize, direction: isize) -> usize {
    let count = scene.spec.menu.entries.len() as isize;
    for distance in 1..=count {
        let index = (current as isize + direction * distance).rem_euclid(count) as usize;
        if scene.spec.menu.entries[index].enabled {
            return index;
        }
    }
    current
}

fn map_pointer(x: f32, y: f32, window: (usize, usize), design: (usize, usize)) -> (f32, f32) {
    let scale = (window.0 as f32 / design.0 as f32).min(window.1 as f32 / design.1 as f32);
    let offset_x = (window.0 as f32 - design.0 as f32 * scale) * 0.5;
    let offset_y = (window.1 as f32 - design.1 as f32 * scale) * 0.5;
    ((x - offset_x) / scale, (y - offset_y) / scale)
}
