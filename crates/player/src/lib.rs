#![forbid(unsafe_code)]
//! Filesystem loading and native presentation for KeyGen scenes.

use keygen_engine::{
    model::SceneSpec, project::ProjectManifest, runtime::ProjectRouteNavigator, Scene, SceneAssets,
};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub mod host;
pub mod native;
pub mod storage;
pub mod story;
use story::{StoryHost, StoryView};

/// Resolve a launcher route to its packaged scene document.
///
/// The project manifest is the declaration of the mapping (`route.scene`),
/// while the package convention keeps scene documents in `scenes/`.  We only
/// accept a single safe filename and never follow route-provided directories;
/// this keeps a malformed package from escaping its package root.
pub fn resolve_route_scene(
    project_root: &Path,
    project: &ProjectManifest,
    route_id: &str,
) -> Result<PathBuf, String> {
    let route = project
        .routes
        .iter()
        .find(|route| route.id == route_id)
        .ok_or_else(|| format!("unknown project route: {route_id}"))?;
    let declared = route.scene.as_str();
    if declared.is_empty()
        || declared.contains('/')
        || declared.contains('\\')
        || declared.contains('\0')
        || Path::new(declared).is_absolute()
    {
        return Err(format!("route {} has unsafe scene mapping", route.id));
    }
    let stem = declared.strip_prefix("scene.").unwrap_or(declared);
    let filename = if stem.ends_with(".json") {
        stem.to_owned()
    } else {
        format!("{stem}.json")
    };
    let candidate = project_root.join("scenes").join(filename);
    if !candidate.is_file() {
        return Err(format!(
            "route {} scene not found: {}",
            route.id,
            candidate.display()
        ));
    }
    Ok(candidate)
}

/// Resolve and load a route's legacy renderable scene document.
pub fn load_route_scene(
    project_root: &Path,
    project: &ProjectManifest,
    route_id: &str,
) -> Result<Scene, String> {
    let path = resolve_route_scene(project_root, project, route_id)?;
    load_scene(&path)
}

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
    let mut fonts = HashMap::new();
    for path in spec
        .text_layers
        .iter()
        .filter_map(|layer| layer.font_path.as_ref())
    {
        if !fonts.contains_key(path) {
            fonts.insert(path.clone(), read_asset(path, "alternate font")?);
        }
    }
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
            fonts,
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
    for text in &mut spec.text_layers {
        if let Some(path) = &mut text.font_path {
            *path = resolve_path(base, path);
        }
    }
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
            scene
                .spec
                .menu
                .as_ref()
                .map_or(0, |menu| menu.entries.len())
        );
        return Ok(());
    }
    if let Some(output) = args.render {
        write_png(&output, &scene.render(args.time, 0).encode_png()?)?;
        println!("rendered {}", output.display());
        return Ok(());
    }
    let navigator = packaged_navigator(&args.scene);
    run_window(scene, args.smoke_seconds, navigator, args.scene)
}

fn packaged_navigator(scene: &Path) -> Option<ProjectRouteNavigator> {
    let package = scene.parent()?.parent()?;
    let project = package.join("project.json");
    ProjectManifest::load(project)
        .ok()
        .and_then(|manifest| ProjectRouteNavigator::new(manifest).ok())
}

fn run_window(
    mut scene: Scene,
    smoke_seconds: Option<f32>,
    mut navigator: Option<ProjectRouteNavigator>,
    scene_path: PathBuf,
) -> Result<(), String> {
    native::require_supported_host()?;
    let width = scene.spec.design_width as usize;
    let height = scene.spec.design_height as usize;
    let display_size = scene
        .spec
        .fit_window_to_display
        .then(native::active_display_size)
        .transpose()?
        .flatten();
    let window_width = display_size
        .map(|size| size.0)
        .or(scene.spec.window_width.map(|value| value as usize))
        .unwrap_or(width);
    let window_height = display_size
        .map(|size| size.1)
        .or(scene.spec.window_height.map(|value| value as usize))
        .unwrap_or(height);
    let mut window = Window::new(
        &scene.spec.title,
        window_width,
        window_height,
        native::window_options(scene.spec.borderless),
    )
    .map_err(|error| format!("cannot create native window: {error}"))?;
    if scene.spec.borderless {
        window.set_position(0, 0);
    }
    window.set_target_fps(60);
    let start = Instant::now();
    let mut focused = first_enabled(&scene, 0);
    let mut mouse_was_down = false;
    let mut pressed_entry = None;
    let package_root = scene_path.parent().and_then(Path::parent);
    let mut story_host = package_root.and_then(|root| StoryHost::load(root).ok());
    let mut story_choice: Option<usize> = None;
    let mut displayed_minute = None;
    if let Some(router) = navigator.as_mut() {
        router.advance_boot();
    }
    while window.is_open() {
        let elapsed = start.elapsed().as_secs_f32();
        refresh_system_clock(&mut scene, &mut displayed_minute);
        if smoke_seconds.is_some_and(|seconds| elapsed >= seconds) {
            break;
        }
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            if scene.spec.menu.is_none() {
                if let (Some(router), Some(root)) = (navigator.as_mut(), package_root) {
                    router.back();
                    scene = load_scene(&root.join("scenes/boot.json"))?;
                    focused = first_enabled(&scene, 0);
                    pressed_entry = None;
                    story_choice = None;
                    continue;
                }
            } else {
                if let Some(router) = navigator.as_mut() {
                    router.close();
                }
                break;
            }
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
            focused = next_enabled(&scene, focused, 1);
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
            focused = next_enabled(&scene, focused, -1);
        }
        let window_size = window.get_size();
        let hovered = if let Some((mouse_x, mouse_y)) = window.get_mouse_pos(MouseMode::Discard) {
            let (design_x, design_y) =
                native::map_pointer((mouse_x, mouse_y), window_size, (width, height));
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
            if let Some(entry) = scene
                .spec
                .menu
                .as_ref()
                .and_then(|menu| menu.entries.get(focused))
            {
                println!("menu action: {}", entry.id);
                if entry.id == "exit" {
                    if let Some(router) = navigator.as_mut() {
                        router.close();
                    }
                    break;
                }
                if let Some(router) = navigator.as_mut() {
                    match router.activate(&entry.id) {
                        Ok(route) => {
                            let package_root = scene_path
                                .parent()
                                .and_then(Path::parent)
                                .ok_or("scene is not inside a package")?;
                            match load_route_scene(package_root, router.project(), &entry.id) {
                                Ok(next)
                                    if next.spec.design_width as usize == width
                                        && next.spec.design_height as usize == height =>
                                {
                                    scene = next;
                                    focused = first_enabled(&scene, 0);
                                    pressed_entry = None;
                                    println!("route transition: {route:?}");
                                    if matches!(route, keygen_engine::runtime::Route::Story { .. })
                                    {
                                        if let Some(host) = story_host.as_mut() {
                                            match host.advance() {
                                                Ok(frame) => {
                                                    apply_story_frame(&mut scene, &frame);
                                                    story_choice = print_story_frame(&frame)
                                                }
                                                Err(error) => println!("story error: {error}"),
                                            }
                                        }
                                    }
                                }
                                Ok(next) => {
                                    router.back();
                                    println!(
                                        "route unavailable for {}: scene dimensions {}x{} do not match window {}x{}",
                                        entry.id,
                                        next.spec.design_width,
                                        next.spec.design_height,
                                        width,
                                        height
                                    );
                                }
                                Err(error) => {
                                    router.back();
                                    println!("route unavailable for {}: {error}", entry.id);
                                }
                            }
                        }
                        Err(error) => println!("route unavailable for {}: {error}", entry.id),
                    }
                }
            } else if let Some(host) = story_host.as_mut() {
                let result = if let Some(choice) = story_choice.take() {
                    host.select(choice).and_then(|_| host.advance())
                } else {
                    host.advance()
                };
                match result {
                    Ok(frame) => {
                        apply_story_frame(&mut scene, &frame);
                        story_choice = print_story_frame(&frame)
                    }
                    Err(error) => println!("story input error: {error}"),
                }
            }
        }
        let frame = scene.render(elapsed, focused).packed_rgb();
        window
            .update_with_buffer(&frame, width, height)
            .map_err(|error| format!("native frame failed: {error}"))?;
    }
    Ok(())
}

fn refresh_system_clock(scene: &mut Scene, displayed_minute: &mut Option<u64>) {
    let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return;
    };
    let minute = elapsed.as_secs() / 60;
    if *displayed_minute == Some(minute) {
        return;
    }
    let Some(local) = local_clock_24h() else {
        return;
    };
    for layer in &mut scene.spec.text_layers {
        if layer.system_clock_24h {
            layer.text.clone_from(&local);
        }
    }
    *displayed_minute = Some(minute);
}

/// Read the host's local clock at the I/O boundary. The compositor itself
/// remains deterministic and receives this as ordinary projected text.
#[cfg(unix)]
fn local_clock_24h() -> Option<String> {
    let output = Command::new("/bin/date").arg("+%H:%M").output().ok()?;
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (output.status.success()
        && value.len() == 5
        && value.as_bytes()[2] == b':'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 2 && byte == b':' || index != 2 && byte.is_ascii_digit()))
    .then(|| value.to_owned())
}

#[cfg(not(unix))]
fn local_clock_24h() -> Option<String> {
    None
}

fn print_story_frame(frame: &story::StoryFrame) -> Option<usize> {
    let is_choice = matches!(frame.view, StoryView::Choice(_));
    match &frame.view {
        StoryView::Dialogue(dialogue) => println!("story dialogue: {}", dialogue.text),
        StoryView::Choice(choice) => {
            println!(
                "story choice: {} [{}]",
                choice.prompt,
                choice.entries.join(" | ")
            );
        }
        StoryView::Effects => println!("story effects: {}", frame.effects.len()),
        StoryView::Complete => println!("story complete"),
    }
    if is_choice {
        Some(0)
    } else {
        None
    }
}

fn apply_story_frame(scene: &mut Scene, frame: &story::StoryFrame) {
    let text = match &frame.view {
        StoryView::Dialogue(dialogue) => dialogue.text.clone(),
        StoryView::Choice(choice) => format!("{}\n{}", choice.prompt, choice.entries.join("\n")),
        StoryView::Effects | StoryView::Complete => return,
    };
    if let Some(layer) = scene.spec.text_layers.first_mut() {
        layer.text = text;
    }
}

fn first_enabled(scene: &Scene, start: usize) -> usize {
    scene
        .spec
        .menu
        .as_ref()
        .and_then(|menu| {
            menu.entries
                .iter()
                .enumerate()
                .skip(start)
                .find_map(|(index, entry)| entry.enabled.then_some(index))
        })
        .unwrap_or(0)
}

fn next_enabled(scene: &Scene, current: usize, direction: isize) -> usize {
    let Some(menu) = &scene.spec.menu else {
        return current;
    };
    let count = menu.entries.len() as isize;
    if count == 0 {
        return current;
    }
    for distance in 1..=count {
        let index = (current as isize + direction * distance).rem_euclid(count) as usize;
        if menu.entries[index].enabled {
            return index;
        }
    }
    current
}

#[cfg(test)]
mod route_scene_tests {
    use super::resolve_route_scene;
    use keygen_engine::project::{
        ProjectAsset, ProjectIdentity, ProjectManifest, ProjectRoute, ProjectScene, Viewport,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn project(scene: &str) -> ProjectManifest {
        ProjectManifest {
            schema: keygen_engine::project::SCHEMA.into(),
            project: ProjectIdentity {
                id: "test".into(),
                display_name: "Test".into(),
                version: "1".into(),
            },
            viewport: Viewport {
                width: 1,
                height: 1,
            },
            assets: vec![ProjectAsset {
                id: "asset".into(),
                kind: "image".into(),
                logical_path: "asset.png".into(),
                sha256: "0".repeat(64),
            }],
            scenes: vec![ProjectScene {
                id: scene.into(),
                asset_ids: vec!["asset".into()],
            }],
            routes: vec![ProjectRoute {
                id: "start".into(),
                scene: scene.into(),
                story_entry: None,
            }],
            story: None,
            persistence: Default::default(),
        }
    }

    #[test]
    fn resolves_declared_scene_id_to_adjacent_json() {
        let root = std::env::temp_dir().join(format!(
            "keygen-route-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("scenes")).unwrap();
        fs::write(root.join("scenes/boot.json"), b"{}").unwrap();
        assert_eq!(
            resolve_route_scene(&root, &project("boot"), "start").unwrap(),
            root.join("scenes/boot.json")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn route_resolution_uses_basename_without_leaking_outside_package() {
        let root = std::env::temp_dir().join(format!(
            "keygen-route-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("scenes")).unwrap();
        fs::write(root.join("scenes/boot.json"), b"{}").unwrap();
        assert!(resolve_route_scene(&root, &project("../boot"), "start").is_err());
        assert!(resolve_route_scene(&root, &project("scene.boot"), "start").is_ok());
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod clock_tests {
    use super::local_clock_24h;

    #[cfg(unix)]
    #[test]
    fn host_clock_is_strict_24_hour_text() {
        let value = local_clock_24h().expect("/bin/date should provide local time");
        assert_eq!(value.len(), 5);
        assert_eq!(&value[2..3], ":");
        assert!(value[..2].parse::<u8>().is_ok_and(|hour| hour < 24));
        assert!(value[3..].parse::<u8>().is_ok_and(|minute| minute < 60));
    }
}
