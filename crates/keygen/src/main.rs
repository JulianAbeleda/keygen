//! Generic project CLI.  Game-specific packages are data supplied through a
//! `keygen.project.v1` manifest; this binary contains no title-specific logic.
use keygen_engine::project::ProjectManifest;
use keygen_engine::runtime::{AppCoordinator, SessionInput, SessionManifest};
use keygen_engine::story::{Block, Command, Program, Tag, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn usage() -> &'static str {
    "usage:\n  keygen inspect PROJECT\n  keygen validate PROJECT\n  keygen load PROJECT\n  keygen render PROJECT --scene FILE --output PNG [--time SECONDS]\n  keygen e2e PROJECT\n\nlegacy scene mode:\n  keygen --scene FILE [--render PNG] [--time SECONDS] [--smoke-seconds SECONDS] [--validate]"
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("keygen: {error}\n\n{}", usage());
        std::process::exit(2);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return launch_packaged_app();
    };
    if command.starts_with('-') {
        // Keep the original scene CLI exactly available while the generic
        // project commands become the canonical entry point.
        return keygen_player::run_cli(std::iter::once("keygen".to_owned()).chain(args));
    }
    match command {
        "inspect" => project_summary(project(&args, 1)?, false),
        "validate" => {
            let manifest = project(&args, 1)?;
            manifest.validate()?;
            println!(
                "project OK: {} ({})",
                manifest.project.display_name, manifest.project.id
            );
            Ok(())
        }
        "load" => project_summary(project(&args, 1)?, true),
        "render" => render_project(&args),
        "e2e" => e2e_project(&args),
        "help" | "--help" | "-h" => Err(usage().into()),
        other => Err(format!("unknown command: {other}")),
    }
}

/// Launch the canonical scene when Finder/LaunchServices starts a packaged
/// application without command-line arguments.  The bundle layout is part of
/// the generic KeyGen contract; it does not encode any particular game.
fn launch_packaged_app() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate KeyGen executable: {error}"))?;
    let macos_dir = executable
        .parent()
        .ok_or("KeyGen executable has no parent directory")?;
    let contents = macos_dir
        .parent()
        .ok_or("KeyGen executable is not inside an app bundle")?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return Err(
            "no command supplied; run KeyGen inside a .app bundle or use `keygen --scene FILE`"
                .into(),
        );
    }
    let package = contents.join("Resources").join("package");
    let project = package.join("project.json");
    let scene = package.join("scenes").join("boot.json");
    if !project.is_file() {
        return Err(format!("packaged app is missing {}", project.display()));
    }
    if !scene.is_file() {
        return Err(format!(
            "packaged app is missing canonical boot scene {}",
            scene.display()
        ));
    }
    let manifest = ProjectManifest::load(&project)?;
    manifest.validate()?;
    keygen_player::run_cli([
        "keygen".to_owned(),
        "--scene".to_owned(),
        scene.to_string_lossy().into_owned(),
    ])
}

fn e2e_project(args: &[String]) -> Result<(), String> {
    let manifest = project(args, 1)?;
    let project_manifest = SessionManifest {
        schema: "keygen.project.v1".into(),
        id: manifest.project.id.clone(),
        display_name: manifest.project.display_name.clone(),
        entry_program: manifest
            .story
            .as_ref()
            .map(|s| s.entry.clone())
            .unwrap_or_else(|| "main".into()),
        assets: manifest
            .assets
            .iter()
            .map(|a| (a.id.clone(), a.logical_path.clone()))
            .collect(),
        capabilities: vec![],
    };
    let program = Program {
        schema: "keygen.story.v1".into(),
        blocks: vec![Block {
            id: "main".into(),
            commands: vec![Command {
                tag: Tag::Dialog,
                args: [("text".into(), Value::String("KeyGen session ready".into()))]
                    .into_iter()
                    .collect(),
            }],
        }],
        labels: [("main".into(), 0)].into_iter().collect::<BTreeMap<_, _>>(),
    };
    let mut app = AppCoordinator::new();
    app.advance_boot(1);
    app.launch_story(project_manifest, program)?;
    let output = app.step_story(SessionInput::Activate)?;
    println!(
        "e2e OK: {} → {:?} ({} effects)",
        manifest.project.id,
        app.phase,
        output.effects.len()
    );
    app.close();
    Ok(())
}

fn project(args: &[String], index: usize) -> Result<ProjectManifest, String> {
    let path = args.get(index).ok_or("PROJECT is required")?;
    ProjectManifest::load(path)
}

fn project_summary(manifest: ProjectManifest, loaded: bool) -> Result<(), String> {
    println!(
        "{} {}",
        manifest.project.display_name, manifest.project.version
    );
    println!(
        "id: {}\nviewport: {}x{}\nassets: {}\nscenes: {}\nstory: {}",
        manifest.project.id,
        manifest.viewport.width,
        manifest.viewport.height,
        manifest.assets.len(),
        manifest.scenes.len(),
        manifest.story.as_ref().map_or("none", |_| "configured")
    );
    if loaded {
        println!("loaded: keygen.project.v1");
    }
    Ok(())
}

fn render_project(args: &[String]) -> Result<(), String> {
    let manifest = project(args, 1)?;
    let mut scene = None;
    let mut output = None;
    let mut time = 4.25_f32;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--scene" => {
                i += 1;
                scene = Some(PathBuf::from(args.get(i).ok_or("--scene needs FILE")?));
            }
            "--output" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).ok_or("--output needs PNG")?));
            }
            "--time" => {
                i += 1;
                time = args
                    .get(i)
                    .ok_or("--time needs SECONDS")?
                    .parse()
                    .map_err(|_| "invalid --time")?;
            }
            flag => return Err(format!("unknown render option: {flag}")),
        }
        i += 1;
    }
    let scene = scene.ok_or("--scene is required")?;
    let output = output.ok_or("--output is required")?;
    let loaded = keygen_player::load_scene(&scene)?;
    if loaded.spec.design_width != manifest.viewport.width
        || loaded.spec.design_height != manifest.viewport.height
    {
        return Err("scene viewport does not match project manifest".into());
    }
    keygen_player::write_png(&output, &loaded.render(time, 0).encode_png()?)?;
    println!("rendered {} for {}", output.display(), manifest.project.id);
    Ok(())
}

#[allow(dead_code)]
fn _project_path(path: &Path) -> &Path {
    path
}
