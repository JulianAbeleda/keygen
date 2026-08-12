//! Product-independent deterministic story schema and virtual machine (KGD-400..411).
//!
//! This module deliberately contains no file, window, audio, or platform access. Hosts
//! provide capabilities and consume the typed effects emitted by [`Vm::step`].
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub schema: String,
    pub blocks: Vec<Block>,
    pub labels: BTreeMap<String, usize>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Block {
    pub id: String,
    pub commands: Vec<Command>,
}

/// The complete versioned descriptor vocabulary. Adding a tag is a schema change.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    Dialog,
    Window,
    WindowAuto,
    Text,
    WaitForScreen,
    MenuInput,
    Show,
    Hide,
    Scene,
    LoadImage,
    Size,
    With,
    Immediate,
    Ease,
    Time,
    Pause,
    Play,
    Stop,
    Queue,
    Fade,
    Loop,
    If,
    Line,
    Timeout,
    LoopStart,
    LoopEnd,
    Fork,
    Set,
    Add,
    Random,
    Unlock,
    Clear,
    Nop,
    Call,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Command {
    pub tag: Tag,
    #[serde(default)]
    pub args: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Number(f64),
    String(String),
    List(Vec<Value>),
}
impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let Self::Int(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct State {
    pub globals: BTreeMap<String, Value>,
    pub persistent: BTreeMap<String, Value>,
    pub locals: Vec<BTreeMap<String, Value>>,
    pub cursor: Cursor,
    pub waiting: Option<Wait>,
    pub clock: u64,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Cursor {
    pub block: usize,
    pub ip: usize,
    pub calls: Vec<(usize, usize)>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Wait {
    pub until: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Effect {
    Dialog {
        text: String,
    },
    Screen {
        name: String,
        visible: bool,
    },
    Image {
        name: String,
        visible: bool,
    },
    Audio {
        action: String,
        name: String,
    },
    Set {
        key: String,
        value: Value,
    },
    Capability {
        id: String,
        args: BTreeMap<String, Value>,
    },
    Yield {
        reason: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TraceEvent {
    pub clock: u64,
    pub block: usize,
    pub ip: usize,
    pub tag: Tag,
    pub effects: Vec<Effect>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Snapshot {
    pub state: State,
    pub trace: Vec<TraceEvent>,
}

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    ids: std::collections::BTreeSet<String>,
}
impl Capabilities {
    pub fn register(&mut self, id: impl Into<String>) {
        self.ids.insert(id.into());
    }
    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
}

pub struct Vm {
    pub program: Program,
    pub state: State,
    pub trace: Vec<TraceEvent>,
    pub capabilities: Capabilities,
    cancelled: bool,
}
impl Vm {
    pub fn new(program: Program) -> Result<Self, String> {
        if program.schema != "keygen.story.v1" {
            return Err("unsupported story schema".into());
        }
        if program.blocks.is_empty() {
            return Err("program has no blocks".into());
        }
        Ok(Self {
            program,
            state: State::default(),
            trace: Vec::new(),
            capabilities: Capabilities::default(),
            cancelled: false,
        })
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state.clone(),
            trace: self.trace.clone(),
        }
    }
    pub fn restore(&mut self, s: Snapshot) {
        self.state = s.state;
        self.trace = s.trace;
        self.cancelled = false;
    }
    pub fn step(&mut self) -> Result<Vec<Effect>, String> {
        if self.cancelled {
            return Ok(vec![]);
        }
        if let Some(w) = &self.state.waiting {
            if self.state.clock < w.until {
                return Ok(vec![Effect::Yield {
                    reason: w.reason.clone(),
                }]);
            }
            self.state.waiting = None
        }
        let c = self
            .program
            .blocks
            .get(self.state.cursor.block)
            .ok_or("invalid block")?;
        if self.state.cursor.ip >= c.commands.len() {
            if let Some((b, ip)) = self.state.cursor.calls.pop() {
                self.state.cursor.block = b;
                self.state.cursor.ip = ip;
                return Ok(vec![]);
            }
            return Ok(vec![]);
        }
        let ip = self.state.cursor.ip;
        let cmd = c.commands[ip].clone();
        self.state.cursor.ip += 1;
        let effects = self.execute(cmd.clone())?;
        self.trace.push(TraceEvent {
            clock: self.state.clock,
            block: self.state.cursor.block,
            ip,
            tag: cmd.tag,
            effects: effects.clone(),
        });
        Ok(effects)
    }
    fn execute(&mut self, c: Command) -> Result<Vec<Effect>, String> {
        let a = c.args;
        let get = |k: &str| {
            a.get(k)
                .cloned()
                .ok_or_else(|| format!("missing argument {k}"))
        };
        match c.tag {
            Tag::Dialog | Tag::Text => Ok(vec![Effect::Dialog {
                text: get("text")?.into_string()?,
            }]),
            Tag::Window | Tag::WindowAuto => Ok(vec![Effect::Screen {
                name: get("name")?.into_string()?,
                visible: true,
            }]),
            Tag::Show | Tag::Scene | Tag::LoadImage => Ok(vec![Effect::Image {
                name: get("name")?.into_string()?,
                visible: true,
            }]),
            Tag::Hide => Ok(vec![Effect::Image {
                name: get("name")?.into_string()?,
                visible: false,
            }]),
            Tag::Play | Tag::Queue => Ok(vec![Effect::Audio {
                action: "play".into(),
                name: get("name")?.into_string()?,
            }]),
            Tag::Stop => Ok(vec![Effect::Audio {
                action: "stop".into(),
                name: get("name")?.into_string()?,
            }]),
            Tag::Set | Tag::Add => {
                let k = get("key")?.into_string()?;
                let v = get("value")?;
                self.state.globals.insert(k.clone(), v.clone());
                Ok(vec![Effect::Set { key: k, value: v }])
            }
            Tag::Pause | Tag::Time | Tag::Timeout => {
                let n = get("ticks")?.as_i64().ok_or("expected integer ticks")?;
                if n < 0 {
                    return Err("negative wait".into());
                }
                self.state.waiting = Some(Wait {
                    until: self.state.clock + n as u64,
                    reason: format!("{:?}", c.tag),
                });
                Ok(vec![Effect::Yield {
                    reason: "wait".into(),
                }])
            }
            Tag::Call => {
                let name = get("label")?.into_string()?;
                let b = *self.program.labels.get(&name).ok_or("invalid label")?;
                self.state
                    .cursor
                    .calls
                    .push((self.state.cursor.block, self.state.cursor.ip));
                self.state.cursor.block = b;
                self.state.cursor.ip = 0;
                Ok(vec![])
            }
            Tag::If => {
                if !get("condition")?
                    .as_bool()
                    .ok_or("expected boolean condition")?
                {
                    self.state.cursor.ip += 1;
                }
                Ok(vec![])
            }
            Tag::Unlock
            | Tag::Clear
            | Tag::Random
            | Tag::With
            | Tag::Immediate
            | Tag::Ease
            | Tag::Size
            | Tag::Fade
            | Tag::Loop
            | Tag::LoopStart
            | Tag::LoopEnd
            | Tag::Fork
            | Tag::Line
            | Tag::WaitForScreen
            | Tag::MenuInput
            | Tag::Nop => Ok(vec![Effect::Capability {
                id: format!("{:?}", c.tag),
                args: a,
            }]),
        }
    }
}
impl Value {
    fn into_string(self) -> Result<String, String> {
        if let Value::String(v) = self {
            Ok(v)
        } else {
            Err("expected string".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_tags_serialize() {
        let tags = vec![
            Tag::Dialog,
            Tag::Window,
            Tag::WindowAuto,
            Tag::Text,
            Tag::WaitForScreen,
            Tag::MenuInput,
            Tag::Show,
            Tag::Hide,
            Tag::Scene,
            Tag::LoadImage,
            Tag::Size,
            Tag::With,
            Tag::Immediate,
            Tag::Ease,
            Tag::Time,
            Tag::Pause,
            Tag::Play,
            Tag::Stop,
            Tag::Queue,
            Tag::Fade,
            Tag::Loop,
            Tag::If,
            Tag::Line,
            Tag::Timeout,
            Tag::LoopStart,
            Tag::LoopEnd,
            Tag::Fork,
            Tag::Set,
            Tag::Add,
            Tag::Random,
            Tag::Unlock,
            Tag::Clear,
            Tag::Nop,
            Tag::Call,
        ];
        assert_eq!(tags.len(), 34);
        for t in tags {
            let s = serde_json::to_string(&t).unwrap();
            assert!(!s.is_empty());
        }
    }
    #[test]
    fn typed_set_and_snapshot() {
        let mut a = BTreeMap::new();
        a.insert("key".into(), Value::String("x".into()));
        a.insert("value".into(), Value::Int(2));
        let mut v = Vm::new(Program {
            schema: "keygen.story.v1".into(),
            blocks: vec![Block {
                id: "b".into(),
                commands: vec![Command {
                    tag: Tag::Set,
                    args: a,
                }],
            }],
            labels: BTreeMap::new(),
        })
        .unwrap();
        v.step().unwrap();
        assert_eq!(v.state.globals["x"], Value::Int(2));
        let s = v.snapshot();
        v.restore(s.clone());
        assert_eq!(v.snapshot(), s);
    }
}
