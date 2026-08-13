//! Host-facing adapter for the packaged `keygen.story.v1` program.
//!
//! This is deliberately presentation-neutral: a window host consumes the returned
//! dialogue, choice, and side-effect values while the deterministic VM remains in
//! `keygen-engine`.
use keygen_engine::story::{Effect, Program, Value, Vm};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryDialogue {
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryChoice {
    pub prompt: String,
    pub entries: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoryView {
    Dialogue(StoryDialogue),
    Choice(StoryChoice),
    Effects,
    Complete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryFrame {
    pub view: StoryView,
    /// Non-dialogue effects are retained for the host compositor/audio adapter.
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug)]
struct PendingChoice {
    labels: Vec<String>,
}

/// Deterministic story presenter. It performs no I/O after construction and is
/// therefore usable by both the native window and headless qualification tools.
pub struct PackagedStory {
    pub vm: Vm,
    pending: Option<PendingChoice>,
}

impl PackagedStory {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes =
            fs::read(path).map_err(|e| format!("cannot read story {}: {e}", path.display()))?;
        let program: Program = serde_json::from_slice(&bytes)
            .map_err(|e| format!("cannot parse story {}: {e}", path.display()))?;
        Self::new(program)
    }

    pub fn new(program: Program) -> Result<Self, String> {
        Ok(Self {
            vm: Vm::new(program)?,
            pending: None,
        })
    }

    pub fn start(&mut self, label: &str) -> Result<(), String> {
        self.vm.state.cursor.block = *self
            .vm
            .program
            .labels
            .get(label)
            .ok_or_else(|| format!("unknown story label: {label}"))?;
        self.vm.state.cursor.ip = 0;
        self.pending = None;
        Ok(())
    }

    /// Advance until the next user-visible dialogue, choice, or completion.
    pub fn advance(&mut self) -> Result<StoryFrame, String> {
        self.pending = None;
        for _ in 0..1024 {
            let effects = self.vm.step()?;
            if effects.is_empty() {
                if self.vm.state.cursor.calls.is_empty()
                    && self
                        .vm
                        .program
                        .blocks
                        .get(self.vm.state.cursor.block)
                        .is_some_and(|b| self.vm.state.cursor.ip >= b.commands.len())
                {
                    return Ok(StoryFrame {
                        view: StoryView::Complete,
                        effects,
                    });
                }
                continue;
            }
            let mut side_effects = Vec::new();
            for effect in effects {
                match effect {
                    Effect::Dialog { text } => {
                        return Ok(StoryFrame {
                            view: StoryView::Dialogue(StoryDialogue { text }),
                            effects: side_effects,
                        })
                    }
                    Effect::Capability { id, args }
                        if id.replace('_', "").eq_ignore_ascii_case("menuinput") =>
                    {
                        let (choice, labels) = parse_choice(args)?;
                        self.pending = Some(PendingChoice { labels });
                        return Ok(StoryFrame {
                            view: StoryView::Choice(choice),
                            effects: side_effects,
                        });
                    }
                    other => side_effects.push(other),
                }
            }
            if !side_effects.is_empty() {
                return Ok(StoryFrame {
                    view: StoryView::Effects,
                    effects: side_effects,
                });
            }
        }
        Err("story exceeded bounded presentation step budget".into())
    }

    pub fn select(&mut self, index: usize) -> Result<(), String> {
        let pending = self
            .pending
            .take()
            .ok_or("story is not awaiting a choice")?;
        let label = pending
            .labels
            .get(index)
            .ok_or("choice index out of range")?;
        self.vm.state.cursor.block = *self
            .vm
            .program
            .labels
            .get(label)
            .ok_or_else(|| format!("choice target label missing: {label}"))?;
        self.vm.state.cursor.ip = 0;
        Ok(())
    }
}

fn parse_choice(mut args: BTreeMap<String, Value>) -> Result<(StoryChoice, Vec<String>), String> {
    let prompt = match args.remove("prompt") {
        Some(Value::String(v)) => v,
        _ => String::new(),
    };
    let entries = match args.remove("entries") {
        Some(Value::List(v)) => v.into_iter().map(value_string).collect::<Vec<_>>(),
        _ => return Err("menu_input requires entries".into()),
    };
    let labels = match args.remove("labels") {
        Some(Value::List(v)) => v.into_iter().map(value_string).collect::<Vec<_>>(),
        _ => return Err("menu_input requires labels".into()),
    };
    if entries.is_empty() || entries.len() != labels.len() {
        return Err("choice entries and labels must align".into());
    }
    Ok((StoryChoice { prompt, entries }, labels))
}
fn value_string(v: Value) -> String {
    match v {
        Value::String(v) => v,
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
    fn s(v: &str) -> Value {
        Value::String(v.into())
    }
    #[test]
    fn presents_dialogue_then_choice_and_selected_branch() {
        let mut labels = BTreeMap::new();
        labels.extend([(String::from("start"), 0), (String::from("left"), 1)]);
        let mut menu = BTreeMap::new();
        menu.insert("prompt".into(), s("Pick"));
        menu.insert("entries".into(), Value::List(vec![s("Left")]));
        menu.insert("labels".into(), Value::List(vec![s("left")]));
        let p = Program {
            schema: "keygen.story.v1".into(),
            labels,
            blocks: vec![
                Block {
                    id: "start".into(),
                    commands: vec![
                        Command {
                            tag: Tag::Dialog,
                            args: [("text".into(), s("Hello"))].into(),
                        },
                        Command {
                            tag: Tag::MenuInput,
                            args: menu,
                        },
                    ],
                },
                Block {
                    id: "left".into(),
                    commands: vec![Command {
                        tag: Tag::Dialog,
                        args: [("text".into(), s("Chosen"))].into(),
                    }],
                },
            ],
        };
        let mut story = PackagedStory::new(p).unwrap();
        story.start("start").unwrap();
        assert!(
            matches!(story.advance().unwrap().view, StoryView::Dialogue(StoryDialogue { ref text }) if text == "Hello")
        );
        assert!(
            matches!(story.advance().unwrap().view, StoryView::Choice(StoryChoice { ref entries, .. }) if entries == &["Left"])
        );
        story.select(0).unwrap();
        assert!(
            matches!(story.advance().unwrap().view, StoryView::Dialogue(StoryDialogue { ref text }) if text == "Chosen")
        );
    }
}
