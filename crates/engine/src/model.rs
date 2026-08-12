use serde::Deserialize;

pub const SCHEMA: &str = "keygen.scene.v1";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSpec {
    pub schema: String,
    pub title: String,
    pub design_width: u32,
    pub design_height: u32,
    pub clear: Color,
    pub font_path: String,
    pub layers: Vec<ImageLayerSpec>,
    #[serde(default)]
    pub particle_insertions: Vec<usize>,
    pub menu_insertion: Option<usize>,
    #[serde(default)]
    pub menu: Option<MenuSpec>,
    #[serde(default)]
    pub text_layers: Vec<TextLayerSpec>,
    pub particles: Option<ParticleSpec>,
    pub fade: Option<FadeSpec>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(transparent)]
pub struct Color(pub [u8; 4]);

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    TopLeft,
    Center,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageLayerSpec {
    pub id: String,
    pub path: String,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub anchor: Anchor,
    #[serde(default = "one")]
    pub alpha: f32,
    pub entrance: Option<EntranceSpec>,
    pub motion: Option<MotionSpec>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    Ease,
    Cubic,
    Quint,
    Bounce,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntranceSpec {
    #[serde(default)]
    pub delay: f32,
    pub duration: f32,
    #[serde(default)]
    pub from_x: f32,
    #[serde(default)]
    pub from_y: f32,
    #[serde(default = "one")]
    pub from_scale: f32,
    pub easing: Easing,
    pub scale_delay: Option<f32>,
    pub scale_duration: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSpec {
    pub period: f32,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuSpec {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub row_height: f32,
    pub spacing: f32,
    pub font_size: f32,
    pub outline_width: u8,
    pub color: Color,
    pub outline: Color,
    pub focused_outline: Color,
    pub entries: Vec<MenuEntrySpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuEntrySpec {
    pub id: String,
    pub label: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextLayerSpec {
    pub id: String,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub color: Color,
    pub outline: Color,
    pub outline_width: u8,
    #[serde(default)]
    pub visible_at: f32,
    pub characters_per_second: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleSpec {
    pub path: String,
    pub origin_x: f32,
    pub origin_y: f32,
    pub count: u32,
    pub start: f32,
    pub lifetime: f32,
    pub bursts: u32,
    pub x_speed: f32,
    pub y_speed: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FadeSpec {
    pub color: Color,
    pub initial_duration: f32,
    pub flash_at: f32,
    pub flash_alpha: f32,
    pub flash_duration: f32,
}

fn one() -> f32 {
    1.0
}

fn enabled() -> bool {
    true
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|value| value.is_finite())
}

impl SceneSpec {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let scene: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid scene document: {error}"))?;
        scene.validate()?;
        Ok(scene)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("unsupported scene schema: {}", self.schema));
        }
        if self.title.trim().is_empty() {
            return Err("scene title cannot be empty".into());
        }
        if self.design_width == 0 || self.design_height == 0 {
            return Err("scene design size must be positive".into());
        }
        let pixels = u64::from(self.design_width) * u64::from(self.design_height);
        if pixels > 16_777_216 {
            return Err("scene design surface is too large".into());
        }
        if self.font_path.trim().is_empty() {
            return Err("font path cannot be empty".into());
        }
        if self.layers.is_empty() {
            return Err("scene needs at least one image layer".into());
        }
        for layer in &self.layers {
            if layer.id.trim().is_empty()
                || layer.path.trim().is_empty()
                || layer.scale <= 0.0
                || !(0.0..=1.0).contains(&layer.alpha)
                || !finite(&[layer.x, layer.y, layer.scale, layer.alpha])
            {
                return Err("layer id, path, geometry, scale, or alpha is invalid".into());
            }
            if let Some(entrance) = &layer.entrance {
                let scale_delay = entrance.scale_delay.unwrap_or(entrance.delay);
                let scale_duration = entrance.scale_duration.unwrap_or(entrance.duration);
                if entrance.duration <= 0.0
                    || entrance.from_scale <= 0.0
                    || scale_duration <= 0.0
                    || !finite(&[
                        entrance.delay,
                        entrance.duration,
                        entrance.from_x,
                        entrance.from_y,
                        entrance.from_scale,
                        scale_delay,
                        scale_duration,
                    ])
                {
                    return Err(format!("layer {} has an invalid entrance", layer.id));
                }
            }
            if let Some(motion) = &layer.motion {
                if motion.period <= 0.0 || !finite(&[motion.period, motion.dx, motion.dy]) {
                    return Err(format!("layer {} has an invalid motion", layer.id));
                }
            }
        }
        if self
            .particle_insertions
            .iter()
            .any(|insertion| *insertion > self.layers.len())
        {
            return Err("particle insertion is outside the layer list".into());
        }
        if self
            .menu_insertion
            .is_some_and(|insertion| insertion > self.layers.len())
        {
            return Err("menu insertion is outside the layer list".into());
        }
        if let Some(menu) = &self.menu {
            if menu.entries.is_empty()
                || menu
                    .entries
                    .iter()
                    .any(|entry| entry.id.trim().is_empty() || entry.label.trim().is_empty())
                || menu.font_size <= 0.0
                || menu.row_height <= 0.0
                || menu.width <= 0.0
                || !finite(&[
                    menu.x,
                    menu.y,
                    menu.width,
                    menu.row_height,
                    menu.spacing,
                    menu.font_size,
                ])
            {
                return Err("menu geometry or entries are invalid".into());
            }
        }
        for text in &self.text_layers {
            if text.id.trim().is_empty()
                || text.font_size <= 0.0
                || text.visible_at < 0.0
                || text
                    .characters_per_second
                    .is_some_and(|value| value <= 0.0 || !value.is_finite())
                || !finite(&[text.x, text.y, text.font_size, text.visible_at])
            {
                return Err(format!("text layer {} is invalid", text.id));
            }
        }
        if let Some(particles) = &self.particles {
            if particles.path.trim().is_empty()
                || particles.count > 512
                || particles.lifetime <= 0.0
                || particles.bursts == 0
                || !finite(&[
                    particles.origin_x,
                    particles.origin_y,
                    particles.start,
                    particles.lifetime,
                    particles.x_speed,
                    particles.y_speed,
                ])
            {
                return Err("particle configuration is invalid".into());
            }
        }
        if let Some(fade) = &self.fade {
            if fade.initial_duration <= 0.0
                || fade.flash_duration <= 0.0
                || !(0.0..=1.0).contains(&fade.flash_alpha)
                || !finite(&[
                    fade.initial_duration,
                    fade.flash_at,
                    fade.flash_alpha,
                    fade.flash_duration,
                ])
            {
                return Err("fade configuration is invalid".into());
            }
        }
        Ok(())
    }
}
