//! Generic project CLI.  Game-specific packages are data supplied through a
//! `keygen.project.v1` manifest; this binary contains no title-specific logic.
use keygen_engine::project::ProjectManifest;
use std::path::{Path, PathBuf};

fn usage() -> &'static str {
    "usage:\n  keygen inspect PROJECT\n  keygen validate PROJECT\n  keygen load PROJECT\n  keygen render PROJECT --scene FILE --output PNG [--time SECONDS]\n\nlegacy scene mode:\n  keygen --scene FILE [--render PNG] [--time SECONDS] [--smoke-seconds SECONDS] [--validate]"
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("keygen: {error}\n\n{}", usage());
        std::process::exit(2);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage().into());
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
        "help" | "--help" | "-h" => Err(usage().into()),
        other => Err(format!("unknown command: {other}")),
    }
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
