//! Product-neutral runtime contracts for KeyGen projects.
//!
//! A project supplies a manifest and story program; hosts supply rendering, audio,
//! filesystem, and platform capabilities. No product identifiers belong here.
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
