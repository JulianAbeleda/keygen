//! Deterministic story-session adapter (KGD-412..425, KGD-444..445).
//!
//! Imported content supplies the [`Program`].  This layer only translates VM effects
//! into VN-facing outputs and projects durable progression into [`SessionState`].
use super::{replay_proof, Cancellation, CapabilityRegistry};
use crate::state::SessionState;
use keygen_engine::story::{Effect, Program, Snapshot, TraceEvent, Value, Vm};
use keygen_player::storage::{AtomicStore, StoreMetadata};
use std::collections::BTreeMap;

pub const RUNTIME_SCHEMA: &str = "keygen.story.session.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogueOutput {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceOutput {
    pub prompt: String,
    pub entries: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionOutput {
    Dialogue(DialogueOutput),
    Choice(ChoiceOutput),
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingChoice {
    entries: Vec<String>,
    labels: Vec<String>,
}

/// A bounded host-facing story session. It never reads files or owns platform resources.
pub struct StorySession {
    pub vm: Vm,
    pub state: SessionState,
    pub capabilities: CapabilityRegistry,
    pending: Option<PendingChoice>,
}

impl StorySession {
    pub fn new(
        program: Program,
        state: SessionState,
        capabilities: CapabilityRegistry,
    ) -> Result<Self, String> {
        Ok(Self {
            vm: Vm::new(program)?,
            state,
            capabilities,
            pending: None,
        })
    }

    pub fn start(&mut self, label: &str) -> Result<(), String> {
        let block = *self
            .vm
            .program
            .labels
            .get(label)
            .ok_or_else(|| format!("unknown story label: {label}"))?;
        self.vm.state.cursor.block = block;
        self.vm.state.cursor.ip = 0;
        self.state.set_variable("story.label", label)?;
        self.pending = None;
        Ok(())
    }

    /// Executes until a user-visible boundary. VM capability effects are dispatched,
    /// while all emitted trace events remain available for replay qualification.
    pub fn advance(&mut self) -> Result<SessionOutput, String> {
        self.pending = None;
        for _ in 0..1024 {
            let before = self.vm.trace.len();
            let effects = self.vm.step()?;
            if effects.is_empty() && self.vm.trace.len() == before {
                return Ok(SessionOutput::Complete);
            }
            for effect in effects {
                match effect {
                    Effect::Dialog { text } => {
                        self.state.record_line(crate::vn::ScreenText {
                            speaker: None,
                            text: text.clone(),
                        });
                        return Ok(SessionOutput::Dialogue(DialogueOutput { text }));
                    }
                    Effect::Capability { id, args } => {
                        let id = super::canonical_id(&id);
                        if id == "menu_input" {
                            let (choice, pending) = parse_choice(args)?;
                            self.pending = Some(pending);
                            return Ok(SessionOutput::Choice(choice));
                        }
                        let request = super::CapabilityRequest { id, args };
                        let response = self
                            .capabilities
                            .dispatch(&request, &Cancellation::default())
                            .map_err(|d| format!("{d:?}"))?;
                        for emitted in response.effects {
                            if let Effect::Set { key, value } = emitted {
                                self.state.set_variable(key, value_to_string(&value))?;
                            }
                        }
                    }
                    Effect::Set { key, value } => {
                        self.state.set_variable(key, value_to_string(&value))?
                    }
                    Effect::Yield { .. }
                    | Effect::Screen { .. }
                    | Effect::Image { .. }
                    | Effect::Audio { .. } => {}
                }
            }
        }
        Err("story session exceeded bounded step budget".into())
    }

    pub fn select(&mut self, index: usize) -> Result<(), String> {
        let pending = self
            .pending
            .take()
            .ok_or("story session is not awaiting a choice")?;
        let label = pending
            .labels
            .get(index)
            .ok_or("choice index out of range")?;
        let block = *self
            .vm
            .program
            .labels
            .get(label)
            .ok_or("choice target label missing")?;
        self.vm.state.cursor.block = block;
        self.vm.state.cursor.ip = 0;
        self.state.set_variable("story.choice", index.to_string())?;
        Ok(())
    }

    pub fn trace(&self) -> &[TraceEvent] {
        &self.vm.trace
    }
    pub fn replay_proof(&self) -> Result<super::ReplayProof, keygen_diagnostics::Diagnostic> {
        replay_proof(self.trace())
    }
    pub fn snapshot(&self) -> Snapshot {
        self.vm.snapshot()
    }
    pub fn save(&mut self, store: &AtomicStore) -> Result<StoreMetadata, String> {
        self.state.revision += 1;
        self.state.save(store)
    }
}

fn parse_choice(
    mut args: BTreeMap<String, Value>,
) -> Result<(ChoiceOutput, PendingChoice), String> {
    let prompt = match args.remove("prompt") {
        Some(Value::String(s)) => s,
        _ => String::new(),
    };
    let values = args
        .remove("entries")
        .ok_or("menu_input requires entries")?;
    let entries: Vec<String> = match values {
        Value::List(items) => items.iter().map(value_to_string).collect(),
        _ => return Err("choice entries must be a list".into()),
    };
    let labels: Vec<String> = match args.remove("labels") {
        Some(Value::List(items)) => items.iter().map(value_to_string).collect(),
        _ => vec![],
    };
    if entries.is_empty() || entries.len() != labels.len() {
        return Err("choice entries and labels must be non-empty and aligned".into());
    }
    Ok((
        ChoiceOutput {
            prompt,
            entries: entries.clone(),
        },
        PendingChoice { entries, labels },
    ))
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(v) => v.to_string(),
        Value::Int(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::Null => "null".into(),
        Value::List(v) => format!("{} values", v.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keygen_engine::story::{Block, Command, Tag};

    fn program() -> Program {
        let mut labels = BTreeMap::new();
        labels.insert("start".into(), 0);
        labels.insert("left".into(), 1);
        labels.insert("right".into(), 2);
        let mut menu = BTreeMap::new();
        menu.insert("prompt".into(), Value::String("choose".into()));
        menu.insert(
            "entries".into(),
            Value::List(vec![
                Value::String("left".into()),
                Value::String("right".into()),
            ]),
        );
        menu.insert(
            "labels".into(),
            Value::List(vec![
                Value::String("left".into()),
                Value::String("right".into()),
            ]),
        );
        Program {
            schema: "keygen.story.v1".into(),
            labels,
            blocks: vec![
                Block {
                    id: "start".into(),
                    commands: vec![Command {
                        tag: Tag::MenuInput,
                        args: menu,
                    }],
                },
                Block {
                    id: "left".into(),
                    commands: vec![Command {
                        tag: Tag::Dialog,
                        args: [("text".into(), Value::String("left line".into()))]
                            .into_iter()
                            .collect(),
                    }],
                },
                Block {
                    id: "right".into(),
                    commands: vec![Command {
                        tag: Tag::Dialog,
                        args: [("text".into(), Value::String("right line".into()))]
                            .into_iter()
                            .collect(),
                    }],
                },
            ],
        }
    }
    #[test]
    fn imported_choice_routes_and_persists() {
        let mut session = StorySession::new(
            program(),
            SessionState::default(),
            CapabilityRegistry::default(),
        )
        .unwrap();
        session.start("start").unwrap();
        let output = session.advance().unwrap();
        assert_eq!(
            output,
            SessionOutput::Choice(ChoiceOutput {
                prompt: "choose".into(),
                entries: vec!["left".into(), "right".into()]
            })
        );
        session.select(1).unwrap();
        assert_eq!(
            session.advance().unwrap(),
            SessionOutput::Dialogue(DialogueOutput {
                text: "right line".into()
            })
        );
        assert_eq!(session.state.variables["story.choice"], "1");
        assert_eq!(session.replay_proof().unwrap().events, 2);
    }
}
