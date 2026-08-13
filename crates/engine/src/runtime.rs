//! Product-neutral runtime contracts for KeyGen projects.
//!
//! A project supplies a manifest and story program; hosts supply rendering, audio,
//! filesystem, and platform capabilities. No product identifiers belong here.
use crate::project::{ProjectManifest, ProjectRoute};
use crate::story::{Effect, Program, Snapshot, Tag, Value, Vm};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionManifest {
    pub schema: String,
    pub id: String,
    pub display_name: String,
    pub entry_program: String,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
    #[serde(default)]
    pub capabilities: Vec<CapabilitySpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySpec {
    pub id: String,
    pub version: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub String);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityCatalog(BTreeMap<CapabilityId, u32>);
impl CapabilityCatalog {
    pub fn from_manifest(manifest: &SessionManifest) -> Result<Self, String> {
        let mut out = Self::default();
        for capability in &manifest.capabilities {
            if capability.id.is_empty() || capability.version == 0 {
                return Err("capability ids must be non-empty and versions positive".into());
            }
            if out
                .0
                .insert(CapabilityId(capability.id.clone()), capability.version)
                .is_some()
            {
                return Err(format!("duplicate capability: {}", capability.id));
            }
        }
        Ok(out)
    }
    pub fn supports(&self, id: &str, minimum_version: u32) -> bool {
        self.0
            .get(&CapabilityId(id.into()))
            .is_some_and(|v| *v >= minimum_version)
    }
    pub fn ids(&self) -> impl Iterator<Item = &CapabilityId> {
        self.0.keys()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Route {
    Boot,
    Launcher,
    Story { entry: String },
    App { id: String },
    Closed,
}

/// Title-neutral navigation over the routes declared by a compiled project.
///
/// Hosts own presentation and input translation; this type owns the route
/// contract so a native window and a headless runner make the same decisions.
/// In particular, a launcher selection is resolved from `project.routes`
/// rather than from filenames or product-specific constants.
#[derive(Clone, Debug)]
pub struct ProjectRouteNavigator {
    project: ProjectManifest,
    route: Route,
}

impl ProjectRouteNavigator {
    pub fn new(project: ProjectManifest) -> Result<Self, String> {
        project.validate()?;
        Ok(Self {
            project,
            route: Route::Boot,
        })
    }

    pub fn project(&self) -> &ProjectManifest {
        &self.project
    }

    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn launcher_routes(&self) -> &[ProjectRoute] {
        &self.project.routes
    }

    pub fn advance_boot(&mut self) {
        if self.route == Route::Boot {
            self.route = Route::Launcher;
        }
    }

    pub fn activate(&mut self, route_id: &str) -> Result<Route, String> {
        if self.route != Route::Launcher {
            return Err("route activation requires the launcher route".into());
        }
        let route = self
            .project
            .routes
            .iter()
            .find(|candidate| candidate.id == route_id)
            .ok_or_else(|| format!("unknown project route: {route_id}"))?;
        self.route = match &route.story_entry {
            Some(entry) => Route::Story {
                entry: entry.clone(),
            },
            None => Route::App {
                id: route.id.clone(),
            },
        };
        Ok(self.route.clone())
    }

    pub fn back(&mut self) {
        match self.route {
            Route::Story { .. } | Route::App { .. } => self.route = Route::Launcher,
            Route::Launcher => self.route = Route::Boot,
            Route::Boot | Route::Closed => {}
        }
    }

    pub fn close(&mut self) {
        self.route = Route::Closed;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum SessionInput {
    Activate,
    Back,
    Select(usize),
    Tick(u64),
    Cancel,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionOutput {
    pub effects: Vec<Effect>,
    pub route: Option<Route>,
    pub finished: bool,
}

pub struct ProjectSession {
    pub manifest: SessionManifest,
    pub vm: Vm,
    pub route: Route,
    assets: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppPhase {
    Boot,
    Launcher,
    Story,
    Closed,
}

/// Product-neutral vertical coordinator used by native and headless hosts.
pub struct AppCoordinator {
    pub phase: AppPhase,
    pub ticks: u64,
    pub session: Option<ProjectSession>,
}

impl AppCoordinator {
    pub fn new() -> Self {
        Self {
            phase: AppPhase::Boot,
            ticks: 0,
            session: None,
        }
    }

    pub fn advance_boot(&mut self, ticks: u64) {
        if self.phase == AppPhase::Boot {
            self.ticks = self.ticks.saturating_add(ticks);
            if self.ticks >= 1 {
                self.phase = AppPhase::Launcher;
            }
        }
    }

    pub fn launch_story(
        &mut self,
        manifest: SessionManifest,
        program: Program,
    ) -> Result<(), String> {
        if self.phase != AppPhase::Launcher {
            return Err("story launch requires the launcher phase".into());
        }
        self.session = Some(ProjectSession::new(manifest, program)?);
        self.phase = AppPhase::Story;
        Ok(())
    }

    pub fn step_story(&mut self, input: SessionInput) -> Result<SessionOutput, String> {
        if self.phase != AppPhase::Story {
            return Err("story step requires the story phase".into());
        }
        let session = self.session.as_mut().ok_or("story session is missing")?;
        let output = session.step(input)?;
        if output.finished {
            self.phase = AppPhase::Launcher;
        }
        Ok(output)
    }

    pub fn close(&mut self) {
        self.session = None;
        self.phase = AppPhase::Closed;
    }
}

impl Default for AppCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProjectSession {
    pub fn new(manifest: SessionManifest, program: Program) -> Result<Self, String> {
        if manifest.schema != "keygen.project.v1" || manifest.id.is_empty() {
            return Err("unsupported or empty project manifest".into());
        }
        if manifest.entry_program.is_empty() {
            return Err("entry program is not declared by project".into());
        }
        let entry = manifest.entry_program.clone();
        let assets = manifest.assets.keys().cloned().collect();
        Ok(Self {
            manifest,
            vm: Vm::new(program)?,
            route: Route::Story { entry },
            assets,
        })
    }
    pub fn asset_declared(&self, id: &str) -> bool {
        self.assets.contains(id)
    }
    pub fn snapshot(&self) -> Snapshot {
        self.vm.snapshot()
    }
    pub fn step(&mut self, input: SessionInput) -> Result<SessionOutput, String> {
        match input {
            SessionInput::Cancel | SessionInput::Back => {
                self.route = Route::Launcher;
                return Ok(SessionOutput {
                    route: Some(self.route.clone()),
                    ..Default::default()
                });
            }
            SessionInput::Tick(clock) => {
                self.vm.state.clock = clock;
            }
            SessionInput::Activate | SessionInput::Select(_) => {}
        }
        let effects = self.vm.step()?;
        Ok(SessionOutput {
            effects,
            route: Some(self.route.clone()),
            finished: self.vm.state.cursor.calls.is_empty()
                && self
                    .vm
                    .program
                    .blocks
                    .get(self.vm.state.cursor.block)
                    .is_some_and(|b| self.vm.state.cursor.ip >= b.commands.len()),
        })
    }
    pub fn register_capabilities(&mut self, catalog: &CapabilityCatalog) {
        for id in catalog.ids() {
            self.vm.capabilities.register(id.0.clone());
        }
    }
    pub fn has_tag(program: &Program, tag: Tag) -> bool {
        program
            .blocks
            .iter()
            .any(|b| b.commands.iter().any(|c| c.tag == tag))
    }
    pub fn value_map() -> BTreeMap<String, Value> {
        BTreeMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{Block, Command};
    fn manifest() -> SessionManifest {
        SessionManifest {
            schema: "keygen.project.v1".into(),
            id: "sample".into(),
            display_name: "Sample".into(),
            entry_program: "main".into(),
            assets: BTreeMap::new(),
            capabilities: vec![CapabilitySpec {
                id: "audio.play".into(),
                version: 1,
            }],
        }
    }

    #[test]
    fn project_routes_drive_explicit_boot_launcher_story_transitions() {
        let project = crate::project::ProjectManifest {
            schema: crate::project::SCHEMA.into(),
            project: crate::project::ProjectIdentity {
                id: "sample.project".into(),
                display_name: "Sample".into(),
                version: "0.1.0".into(),
            },
            viewport: crate::project::Viewport {
                width: 1,
                height: 1,
            },
            assets: vec![],
            scenes: vec![crate::project::ProjectScene {
                id: "scene.start".into(),
                asset_ids: vec![],
            }],
            routes: vec![crate::project::ProjectRoute {
                id: "route.start".into(),
                scene: "scene.start".into(),
                story_entry: Some("start".into()),
            }],
            story: Some(crate::project::ProjectStory {
                entry: "start".into(),
                labels: vec!["start".into()],
            }),
            persistence: Default::default(),
        };
        let mut navigator = ProjectRouteNavigator::new(project).unwrap();
        assert_eq!(navigator.route(), &Route::Boot);
        navigator.advance_boot();
        assert_eq!(navigator.route(), &Route::Launcher);
        assert_eq!(
            navigator.activate("route.start").unwrap(),
            Route::Story {
                entry: "start".into()
            }
        );
        navigator.back();
        assert_eq!(navigator.route(), &Route::Launcher);
    }
    #[test]
    fn validates_capabilities() {
        let c = CapabilityCatalog::from_manifest(&manifest()).unwrap();
        assert!(c.supports("audio.play", 1));
    }
    #[test]
    fn session_is_product_neutral() {
        let p = Program {
            schema: "keygen.story.v1".into(),
            blocks: vec![Block {
                id: "main".into(),
                commands: vec![Command {
                    tag: Tag::Nop,
                    args: BTreeMap::new(),
                }],
            }],
            labels: BTreeMap::new(),
        };
        let s = ProjectSession::new(manifest(), p).unwrap();
        assert_eq!(s.manifest.id, "sample");
    }
}
