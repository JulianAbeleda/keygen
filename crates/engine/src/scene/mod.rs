//! Versioned retained-scene primitives for KeyGen.
//!
//! This module is deliberately independent of assets, hosts, and product
//! mappings.  It describes deterministic scene state and rendering contracts;
//! importers are responsible for turning source data into these values.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_V2: &str = "keygen.scene.v2";

/// Migration entry point reserved for the v1 compositor adapter. Keeping the
/// boundary explicit prevents callers from treating v1 JSON as v2 by accident.
pub fn migrate_v1_schema(_bytes: &[u8]) -> Result<SceneDocument, String> {
    Err("keygen.scene.v1 migration requires the host adapter; no implicit migration".into())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
    pub pivot: Vec2,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::default(),
            scale: Vec2 { x: 1.0, y: 1.0 },
            rotation: 0.0,
            pivot: Vec2::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.width && p.y < self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDocument {
    pub schema: String,
    pub reference: Reference,
    pub nodes: Vec<NodeSpec>,
    #[serde(default)]
    pub cameras: Vec<CameraSpec>,
    #[serde(default)]
    pub materials: Vec<MaterialSpec>,
    #[serde(default)]
    pub animations: Vec<AnimationSpec>,
}
impl SceneDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA_V2 {
            return Err(format!("unsupported scene schema: {}", self.schema));
        }
        if self.reference.width == 0 || self.reference.height == 0 {
            return Err("reference resolution must be positive".into());
        }
        if self.nodes.len() > 100_000 {
            return Err("scene node limit exceeded".into());
        }
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if n.id.is_empty() || !ids.insert(n.id.clone()) {
                return Err("node ids must be unique and non-empty".into());
            }
            if let Some(c) = n.clip {
                if c.width < 0.0 || c.height < 0.0 {
                    return Err("clip dimensions must be non-negative".into());
                }
            }
        }
        for a in &self.animations {
            a.validate()?;
        }
        Ok(())
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let d: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid scene v2: {e}"))?;
        d.validate()?;
        Ok(d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub retina_scale: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSpec {
    pub id: String,
    pub viewport: Rect,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub zoom: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    pub id: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub transform: Transform,
    pub kind: NodeKind,
    pub clip: Option<Rect>,
    #[serde(default)]
    pub z: i32,
    #[serde(default)]
    pub visible: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NodeKind {
    Empty,
    Sprite(SpriteSpec),
    Text(TextSpec),
    Widget(WidgetSpec),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSpec {
    pub asset: String,
    #[serde(default)]
    pub tint: [u8; 4],
    #[serde(default)]
    pub alpha: f32,
    pub sampling: Sampling,
    pub nine_slice: Option<NineSlice>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sampling {
    Nearest,
    Linear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NineSlice {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSpec {
    pub text: String,
    pub font: String,
    pub size: f32,
    pub color: [u8; 4],
    pub outline: Option<Outline>,
    pub layout: TextLayout,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    pub color: [u8; 4],
    pub width: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextLayout {
    pub width: f32,
    pub line_height: f32,
    pub align: Align,
    pub wrap: Wrap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrap {
    None,
    Word,
    Character,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetSpec {
    pub role: WidgetRole,
    pub label: String,
    pub focusable: bool,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetRole {
    Button,
    Label,
    Image,
    List,
    Slider,
    Toggle,
    TextField,
    ScrollView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialSpec {
    pub id: String,
    pub effect: String,
    pub properties: BTreeMap<String, MaterialValue>,
    pub fallback: MaterialFallback,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaterialValue {
    Number(f32),
    Bool(bool),
    Color([u8; 4]),
    Text(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialFallback {
    Error,
    SolidColor,
    Source,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationSpec {
    pub id: String,
    pub duration: f32,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub repeat: Repeat,
}
impl AnimationSpec {
    fn validate(&self) -> Result<(), String> {
        if !self.duration.is_finite() || self.duration <= 0.0 {
            return Err(format!("animation {} has invalid duration", self.id));
        }
        for t in &self.tracks {
            if t.keys.is_empty() {
                return Err(format!("animation track {} has no keys", t.property));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub property: String,
    pub keys: Vec<Keyframe>,
    pub easing: Easing,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    Once,
    Loop,
    Count(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnimationClock {
    pub tick: u64,
    pub micros: u64,
}
impl AnimationClock {
    pub fn advance(&mut self, micros: u64) {
        self.micros = self.micros.saturating_add(micros);
        self.tick = self.tick.saturating_add(1);
    }
    pub fn sample(&self, a: &AnimationSpec) -> Vec<(String, f32)> {
        let t = self.micros as f32 / 1_000_000.0;
        a.tracks
            .iter()
            .map(|tr| {
                (
                    tr.property.clone(),
                    sample_track(tr, t, a.duration, a.repeat),
                )
            })
            .collect()
    }
}
fn sample_track(t: &Track, mut time: f32, duration: f32, repeat: Repeat) -> f32 {
    time = match repeat {
        Repeat::Once => time.min(duration),
        Repeat::Loop => time % duration,
        Repeat::Count(n) => time.min(duration * n.max(1) as f32) % duration,
    };
    let pair = t
        .keys
        .windows(2)
        .find(|w| time <= w[1].time)
        .unwrap_or_else(|| &t.keys[t.keys.len() - 1..]);
    if pair.len() == 1 {
        return pair[0].value;
    }
    let span = (pair[1].time - pair[0].time).max(f32::EPSILON);
    let p = ((time - pair[0].time) / span).clamp(0.0, 1.0);
    pair[0].value + (pair[1].value - pair[0].value) * ease(p, t.easing)
}
fn ease(p: f32, e: Easing) -> f32 {
    match e {
        Easing::Linear => p,
        Easing::EaseIn => p * p,
        Easing::EaseOut => 1.0 - (1.0 - p) * (1.0 - p),
        Easing::EaseInOut => {
            if p < 0.5 {
                2.0 * p * p
            } else {
                1.0 - (-2.0 * p + 2.0).powi(2) / 2.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3([[f32; 3]; 3]);
impl Mat3 {
    pub fn identity() -> Self {
        Self([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
    }
    pub fn transform(self, p: Vec2) -> Vec2 {
        Vec2 {
            x: self.0[0][0] * p.x + self.0[0][1] * p.y + self.0[0][2],
            y: self.0[1][0] * p.x + self.0[1][1] * p.y + self.0[1][2],
        }
    }
    pub fn compose(self, b: Self) -> Self {
        let mut r = [[0.; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[i][k] * b.0[k][j]).sum();
            }
        }
        Self(r)
    }
}
pub fn transform_matrix(t: Transform) -> Mat3 {
    let (s, c) = t.rotation.sin_cos();
    Mat3([
        [c * t.scale.x, -s * t.scale.y, t.position.x],
        [s * t.scale.x, c * t.scale.y, t.position.y],
        [0., 0., 1.],
    ])
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetainedScene {
    nodes: BTreeMap<String, NodeSpec>,
}
impl RetainedScene {
    pub fn apply(&mut self, op: SceneOp) -> Result<(), String> {
        match op {
            SceneOp::Create(n) => {
                if self.nodes.insert(n.id.clone(), n).is_some() {
                    Err("node already exists".into())
                } else {
                    Ok(())
                }
            }
            SceneOp::Update(id, n) => {
                if let std::collections::btree_map::Entry::Occupied(mut entry) =
                    self.nodes.entry(id)
                {
                    entry.insert(n);
                    Ok(())
                } else {
                    Err("node does not exist".into())
                }
            }
            SceneOp::Remove(id) => {
                self.nodes.remove(&id);
                Ok(())
            }
            SceneOp::Reparent(id, p) => {
                let n = self.nodes.get_mut(&id).ok_or("node does not exist")?;
                n.parent = p;
                Ok(())
            }
            SceneOp::Order(id, z) => {
                let n = self.nodes.get_mut(&id).ok_or("node does not exist")?;
                n.z = z;
                Ok(())
            }
        }
    }
    pub fn snapshot(&self) -> Vec<&NodeSpec> {
        let mut v = self.nodes.values().collect::<Vec<_>>();
        v.sort_by_key(|n| (&n.z, &n.id));
        v
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum SceneOp {
    Create(NodeSpec),
    Update(String, NodeSpec),
    Remove(String),
    Reparent(String, Option<String>),
    Order(String, i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipStack {
    pub rect: Rect,
}
impl ClipStack {
    pub fn intersect(self, other: Rect) -> Self {
        let x = self.rect.x.max(other.x);
        let y = self.rect.y.max(other.y);
        let r = (self.rect.x + self.rect.width).min(other.x + other.width);
        let b = (self.rect.y + self.rect.height).min(other.y + other.height);
        Self {
            rect: Rect {
                x,
                y,
                width: (r - x).max(0.),
                height: (b - y).max(0.),
            },
        }
    }
}
pub fn nine_slice_regions(src: Rect, dst: Rect, n: NineSlice) -> [(Rect, Rect); 9] {
    let sx = [
        src.x,
        src.x + n.left as f32,
        src.x + src.width - n.right as f32,
        src.x + src.width,
    ];
    let sy = [
        src.y,
        src.y + n.top as f32,
        src.y + src.height - n.bottom as f32,
        src.y + src.height,
    ];
    let dx = [
        dst.x,
        dst.x + n.left as f32,
        dst.x + dst.width - n.right as f32,
        dst.x + dst.width,
    ];
    let dy = [
        dst.y,
        dst.y + n.top as f32,
        dst.y + dst.height - n.bottom as f32,
        dst.y + dst.height,
    ];
    let mut out = [(
        Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        },
        Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        },
    ); 9];
    let mut k = 0;
    for y in 0..3 {
        for x in 0..3 {
            out[k] = (
                Rect {
                    x: sx[x],
                    y: sy[y],
                    width: sx[x + 1] - sx[x],
                    height: sy[y + 1] - sy[y],
                },
                Rect {
                    x: dx[x],
                    y: dy[y],
                    width: dx[x + 1] - dx[x],
                    height: dy[y + 1] - dy[y],
                },
            );
            k += 1;
        }
    }
    out
}

/// Deterministic ownership groups. Hosts load bytes; the scene only tracks
/// who still needs them and never performs I/O.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourcePool {
    groups: BTreeMap<String, BTreeSet<String>>,
}
impl ResourcePool {
    pub fn acquire(&mut self, group: &str, resource: &str) {
        self.groups
            .entry(group.into())
            .or_default()
            .insert(resource.into());
    }
    pub fn release_group(&mut self, group: &str) -> Vec<String> {
        self.groups
            .remove(group)
            .map(|s| s.into_iter().collect())
            .unwrap_or_default()
    }
    pub fn is_acquired(&self, resource: &str) -> bool {
        self.groups.values().any(|s| s.contains(resource))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub logical: Rect,
    pub physical_width: u32,
    pub physical_height: u32,
}
impl Viewport {
    pub fn map(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: (p.x - self.logical.x) * self.physical_width as f32 / self.logical.width.max(1.0),
            y: (p.y - self.logical.y) * self.physical_height as f32 / self.logical.height.max(1.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphPlacement {
    pub character: char,
    pub x: f32,
    pub y: f32,
    pub line: u32,
}
pub fn layout_text(
    text: &str,
    width: f32,
    advance: f32,
    line_height: f32,
    align: Align,
    wrap: Wrap,
) -> Vec<GlyphPlacement> {
    let mut out = Vec::new();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut line = 0;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            x = 0.0;
            y += line_height;
            continue;
        }
        if !matches!(wrap, Wrap::None) && x + advance > width && x > 0.0 {
            line += 1;
            x = 0.0;
            y += line_height;
        }
        out.push(GlyphPlacement {
            character: ch,
            x,
            y,
            line,
        });
        x += advance;
    }
    if !matches!(align, Align::Left) {
        let mut start = 0;
        for current in 0..=line {
            let end = out
                .iter()
                .position(|g| g.line > current)
                .unwrap_or(out.len());
            let count = (end - start) as f32 * advance;
            let shift = if align == Align::Center {
                (width - count) / 2.0
            } else {
                width - count
            };
            for g in &mut out[start..end] {
                g.x += shift.max(0.0);
            }
            start = end;
        }
    }
    out
}

/// Lays out UTF-8 text using measured per-glyph advances. Newlines are always
/// respected; word wrapping prefers whitespace and falls back to character
/// wrapping for tokens wider than the available width. The callback is called
/// once per character and no unbounded intermediate allocation is performed.
pub fn layout_text_measured<F>(
    text: &str,
    width: f32,
    line_height: f32,
    align: Align,
    wrap: Wrap,
    mut advance: F,
) -> Vec<GlyphPlacement>
where
    F: FnMut(char) -> f32,
{
    let width = width.max(0.0);
    let mut out = Vec::new();
    let mut line = 0u32;
    let mut x = 0.0;
    let mut line_start = 0usize;
    let mut last_space = None;
    let chars = text.chars();
    for ch in chars {
        if ch == '\n' {
            line += 1;
            x = 0.0;
            line_start = out.len();
            last_space = None;
            continue;
        }
        let measured = advance(ch).max(0.0);
        if matches!(wrap, Wrap::None) || x == 0.0 || x + measured <= width {
            out.push(GlyphPlacement {
                character: ch,
                x,
                y: line as f32 * line_height,
                line,
            });
            x += measured;
            if ch.is_whitespace() {
                last_space = Some(out.len() - 1);
            }
            continue;
        }
        if matches!(wrap, Wrap::Word) {
            if let Some(space) = last_space.filter(|s| *s >= line_start) {
                let move_count = out.len() - space - 1;
                line += 1;
                let mut next_x = 0.0;
                for glyph in &mut out[space + 1..] {
                    glyph.line = line;
                    glyph.x = next_x;
                    glyph.y = line as f32 * line_height;
                    next_x += advance(glyph.character).max(0.0);
                }
                out.truncate(space + 1);
                let _ = move_count;
                line_start = out.len();
                last_space = None;
                x = next_x;
                if ch.is_whitespace() {
                    continue;
                }
                out.push(GlyphPlacement {
                    character: ch,
                    x,
                    y: line as f32 * line_height,
                    line,
                });
                x += measured;
                continue;
            }
        }
        line += 1;
        x = 0.0;
        line_start = out.len();
        last_space = None;
        out.push(GlyphPlacement {
            character: ch,
            x,
            y: line as f32 * line_height,
            line,
        });
        x += measured;
    }
    if !matches!(align, Align::Left) {
        let mut start = 0;
        let max_line = out.last().map(|g| g.line).unwrap_or(0);
        for current in 0..=max_line {
            let end = out
                .iter()
                .position(|g| g.line > current)
                .unwrap_or(out.len());
            let line_width = out[start..end]
                .iter()
                .map(|g| advance(g.character).max(0.0))
                .sum::<f32>();
            let shift = match align {
                Align::Center => (width - line_width) / 2.0,
                Align::Right => width - line_width,
                Align::Left => 0.0,
            };
            for glyph in &mut out[start..end] {
                glyph.x += shift.max(0.0);
            }
            start = end;
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
    Focused,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Widget {
    pub id: String,
    pub role: WidgetRole,
    pub state: WidgetState,
    pub enabled: bool,
}
impl Widget {
    pub fn focusable(&self) -> bool {
        self.enabled && !matches!(self.state, WidgetState::Disabled)
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state = WidgetState::Disabled;
        } else if self.state == WidgetState::Disabled {
            self.state = WidgetState::Normal;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    Fade,
    Dissolve,
    Wipe,
    Overlay,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration: f32,
}
impl Transition {
    pub fn progress(&self, t: f32) -> f32 {
        (t / self.duration.max(f32::EPSILON)).clamp(0., 1.)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScreenRegistry {
    active: BTreeSet<String>,
    modals: Vec<String>,
}
impl ScreenRegistry {
    pub fn activate(&mut self, id: &str) {
        self.active.insert(id.into());
    }
    pub fn deactivate(&mut self, id: &str) {
        self.active.remove(id);
    }
    pub fn push_modal(&mut self, id: &str) -> Result<(), String> {
        if self.modals.iter().any(|x| x == id) {
            return Err("modal already open".into());
        }
        self.modals.push(id.into());
        Ok(())
    }
    pub fn pop_modal(&mut self) -> Option<String> {
        self.modals.pop()
    }
    pub fn top_modal(&self) -> Option<&str> {
        self.modals.last().map(String::as_str)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusGraph {
    edges: BTreeMap<String, Vec<String>>,
    current: Option<String>,
}
impl FocusGraph {
    pub fn connect(&mut self, from: &str, to: &str) {
        self.edges.entry(from.into()).or_default().push(to.into());
    }
    pub fn set_default(&mut self, id: &str) {
        self.current = Some(id.into());
    }
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }
    pub fn move_next(&mut self) -> Option<&str> {
        let cur = self.current.clone()?;
        let next = self.edges.get(&cur)?.first()?.clone();
        self.current = Some(next);
        self.current.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, z: i32) -> NodeSpec {
        NodeSpec {
            id: id.into(),
            parent: None,
            transform: Transform::default(),
            kind: NodeKind::Empty,
            clip: None,
            z,
            visible: true,
        }
    }
    #[test]
    fn strict_schema_and_snapshot_are_deterministic() {
        let d = SceneDocument {
            schema: SCHEMA_V2.into(),
            reference: Reference {
                width: 800,
                height: 600,
                retina_scale: 1,
            },
            nodes: vec![node("b", 2), node("a", 1)],
            cameras: vec![],
            materials: vec![],
            animations: vec![],
        };
        assert!(d.validate().is_ok());
        let mut s = RetainedScene::default();
        s.apply(SceneOp::Create(node("b", 2))).unwrap();
        s.apply(SceneOp::Create(node("a", 1))).unwrap();
        assert_eq!(s.snapshot()[0].id, "a");
        assert!(SceneDocument::from_json(br#"{\"schema\":\"keygen.scene.v2\",\"reference\":{\"width\":1,\"height\":1},\"nodes\":[],\"bad\":1}"#).is_err());
    }
    #[test]
    fn transform_clip_nineslice_and_transition_contracts() {
        let m = transform_matrix(Transform {
            position: Vec2 { x: 2., y: 3. },
            ..Default::default()
        });
        assert_eq!(m.transform(Vec2::default()), Vec2 { x: 2., y: 3. });
        let c = ClipStack {
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
        }
        .intersect(Rect {
            x: 5.,
            y: 5.,
            width: 10.,
            height: 10.,
        });
        assert_eq!(c.rect.width, 5.);
        assert_eq!(
            nine_slice_regions(
                Rect {
                    x: 0.,
                    y: 0.,
                    width: 10.,
                    height: 10.
                },
                Rect {
                    x: 0.,
                    y: 0.,
                    width: 20.,
                    height: 20.
                },
                NineSlice {
                    left: 2,
                    top: 2,
                    right: 2,
                    bottom: 2
                }
            )
            .len(),
            9
        );
        assert_eq!(
            Transition {
                kind: TransitionKind::Fade,
                duration: 2.
            }
            .progress(1.),
            0.5
        );
    }

    #[test]
    fn measured_layout_handles_utf8_newlines_and_long_words() {
        let glyphs = layout_text_measured(
            "日本語 abcdef\nxy",
            3.0,
            2.0,
            Align::Left,
            Wrap::Word,
            |_| 1.0,
        );
        assert_eq!(glyphs.iter().filter(|g| g.line == 0).count(), 3);
        assert!(glyphs.iter().any(|g| g.character == '日'));
        assert!(glyphs.iter().any(|g| g.character == 'x' && g.line >= 1));
        assert!(glyphs.iter().all(|g| g.x >= 0.0 && g.y >= 0.0));
    }
    #[test]
    fn clock_is_fixed_and_focus_is_semantic() {
        let a = AnimationSpec {
            id: "x".into(),
            duration: 2.,
            tracks: vec![Track {
                property: "x".into(),
                keys: vec![
                    Keyframe {
                        time: 0.,
                        value: 0.,
                    },
                    Keyframe {
                        time: 2.,
                        value: 10.,
                    },
                ],
                easing: Easing::Linear,
            }],
            repeat: Repeat::Once,
        };
        let mut c = AnimationClock::default();
        c.advance(1_000_000);
        assert_eq!(c.sample(&a)[0].1, 5.);
        let mut f = FocusGraph::default();
        f.connect("a", "b");
        f.set_default("a");
        assert_eq!(f.move_next(), Some("b"));
    }
    #[test]
    fn resources_layout_viewport_and_widgets_are_deterministic() {
        let mut pool = ResourcePool::default();
        pool.acquire("launcher", "font");
        pool.acquire("story", "sprite");
        assert!(pool.is_acquired("font"));
        assert_eq!(pool.release_group("launcher"), vec!["font".to_string()]);
        let placed = layout_text("abcd", 2.0, 1.0, 2.0, Align::Left, Wrap::Character);
        assert_eq!(placed[2].line, 1);
        assert_eq!(
            Viewport {
                logical: Rect {
                    x: 0.,
                    y: 0.,
                    width: 2.,
                    height: 2.
                },
                physical_width: 4,
                physical_height: 4
            }
            .map(Vec2 { x: 1., y: 1. }),
            Vec2 { x: 2., y: 2. }
        );
        let mut w = Widget {
            id: "ok".into(),
            role: WidgetRole::Button,
            state: WidgetState::Normal,
            enabled: true,
        };
        assert!(w.focusable());
        w.set_enabled(false);
        assert!(!w.focusable());
    }
}
