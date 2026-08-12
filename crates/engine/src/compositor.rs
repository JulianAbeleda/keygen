use crate::{
    image::Image,
    model::{Anchor, Color, Easing, EntranceSpec, SceneSpec},
    surface::Surface,
};
use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    Font, FontSettings,
};
use std::collections::HashMap;

pub struct SceneAssets {
    pub font: Vec<u8>,
    pub layers: HashMap<String, Vec<u8>>,
    pub particle: Option<Vec<u8>>,
}

pub struct Scene {
    pub spec: SceneSpec,
    images: HashMap<String, Image>,
    particle: Option<Image>,
    font: Font,
}

impl Scene {
    pub fn from_assets(spec: SceneSpec, assets: SceneAssets) -> Result<Self, String> {
        spec.validate()?;
        let mut images = HashMap::new();
        for layer in &spec.layers {
            let bytes = assets
                .layers
                .get(&layer.id)
                .ok_or_else(|| format!("missing bytes for layer {}", layer.id))?;
            images.insert(layer.id.clone(), Image::decode(&layer.path, bytes)?);
        }
        let particle = match (&spec.particles, assets.particle) {
            (Some(value), Some(bytes)) => Some(Image::decode(&value.path, &bytes)?),
            (Some(_), None) => return Err("missing bytes for particle asset".into()),
            (None, Some(_)) => return Err("particle bytes supplied without particle spec".into()),
            (None, None) => None,
        };
        let font = Font::from_bytes(assets.font, FontSettings::default())
            .map_err(|error| format!("cannot decode font: {error}"))?;
        Ok(Self {
            spec,
            images,
            particle,
            font,
        })
    }

    pub fn render(&self, time: f32, focused: usize) -> Surface {
        let mut surface = Surface::new(
            self.spec.design_width,
            self.spec.design_height,
            self.spec.clear.0,
        );
        self.draw_menu_insertion(&mut surface, focused, 0);
        self.draw_particle_insertions(&mut surface, time, 0);
        for (layer_index, layer) in self.spec.layers.iter().enumerate() {
            let Some(image) = self.images.get(&layer.id) else {
                continue;
            };
            let (mut x, mut y, mut scale) = (layer.x, layer.y, layer.scale);
            if let Some(entrance) = &layer.entrance {
                let progress = transition(time, entrance.delay, entrance.duration, entrance.easing);
                x += entrance.from_x * (1.0 - progress);
                y += entrance.from_y * (1.0 - progress);
                let scale_progress = transition(
                    time,
                    entrance.scale_delay.unwrap_or(entrance.delay),
                    entrance.scale_duration.unwrap_or(entrance.duration),
                    entrance.easing,
                );
                scale *= entrance.from_scale + (1.0 - entrance.from_scale) * scale_progress;
            }
            if let Some(motion) = &layer.motion {
                let phase = time.rem_euclid(motion.period) / motion.period;
                x += motion.dx * phase;
                y += motion.dy * phase;
            }
            let (left, top) = match layer.anchor {
                Anchor::TopLeft => (x, y),
                Anchor::Center => (
                    x - image.width as f32 * scale * 0.5,
                    y - image.height as f32 * scale * 0.5,
                ),
            };
            surface.draw_image(image, left, top, scale, layer.alpha);
            self.draw_menu_insertion(&mut surface, focused, layer_index + 1);
            self.draw_particle_insertions(&mut surface, time, layer_index + 1);
        }
        self.draw_fade(&mut surface, time);
        surface
    }

    pub fn menu_hit(&self, x: f32, y: f32) -> Option<usize> {
        let menu = &self.spec.menu;
        if x < menu.x || x > menu.x + menu.width {
            return None;
        }
        menu.entries.iter().enumerate().find_map(|(index, entry)| {
            let top = menu.y + index as f32 * (menu.row_height + menu.spacing);
            (entry.enabled && y >= top && y < top + menu.row_height).then_some(index)
        })
    }

    fn draw_menu(&self, surface: &mut Surface, focused: usize) {
        let menu = &self.spec.menu;
        for (index, entry) in menu.entries.iter().enumerate() {
            let y = menu.y + index as f32 * (menu.row_height + menu.spacing);
            let outline = if index == focused {
                menu.focused_outline
            } else {
                menu.outline
            };
            self.draw_text(
                surface,
                &entry.label,
                menu.x,
                y,
                menu.font_size,
                menu.color,
                outline,
                menu.outline_width,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &self,
        surface: &mut Surface,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        fill: Color,
        outline: Color,
        radius: u8,
    ) {
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x,
            y,
            ..LayoutSettings::default()
        });
        layout.append(&[&self.font], &TextStyle::new(text, size, 0));
        for glyph in layout.glyphs() {
            let (metrics, bitmap) = self.font.rasterize_config(glyph.key);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }
            let base_x = glyph.x.round() as i32;
            let base_y = glyph.y.round() as i32;
            for oy in -(radius as i32)..=radius as i32 {
                for ox in -(radius as i32)..=radius as i32 {
                    if ox * ox + oy * oy > i32::from(radius) * i32::from(radius) {
                        continue;
                    }
                    draw_mask(
                        surface,
                        &bitmap,
                        metrics.width,
                        base_x + ox,
                        base_y + oy,
                        outline.0,
                    );
                }
            }
            draw_mask(surface, &bitmap, metrics.width, base_x, base_y, fill.0);
        }
    }

    fn draw_particles(&self, surface: &mut Surface, time: f32, instance: u32) {
        let (Some(spec), Some(image)) = (&self.spec.particles, &self.particle) else {
            return;
        };
        for burst in 0..spec.bursts {
            let age = time - spec.start;
            if !(0.0..spec.lifetime).contains(&age) {
                continue;
            }
            let progress = age / spec.lifetime;
            for index in 0..spec.count {
                let seed = index
                    .wrapping_mul(1_103_515_245)
                    .wrapping_add(burst.wrapping_mul(12_345))
                    .wrapping_add(instance.wrapping_mul(2_654_435_761));
                let angle = (seed % 6283) as f32 / 1000.0;
                let speed = 0.45 + ((seed >> 8) % 1000) as f32 / 1800.0;
                let x = spec.origin_x + angle.cos() * spec.x_speed * speed * age * 60.0;
                let y = spec.origin_y + angle.sin() * spec.y_speed * speed * age * 60.0;
                surface.draw_image(image, x, y, 1.0, (1.0 - progress).clamp(0.0, 1.0));
            }
        }
    }

    fn draw_particle_insertions(&self, surface: &mut Surface, time: f32, insertion: usize) {
        let copies = self
            .spec
            .particle_insertions
            .iter()
            .filter(|candidate| **candidate == insertion)
            .count();
        let prior = self
            .spec
            .particle_insertions
            .iter()
            .filter(|candidate| **candidate < insertion)
            .count();
        for copy in 0..copies {
            self.draw_particles(surface, time, (prior + copy) as u32);
        }
    }

    fn draw_menu_insertion(&self, surface: &mut Surface, focused: usize, insertion: usize) {
        if self.spec.menu_insertion.unwrap_or(self.spec.layers.len()) == insertion {
            self.draw_menu(surface, focused);
        }
    }

    fn draw_fade(&self, surface: &mut Surface, time: f32) {
        let Some(fade) = &self.spec.fade else {
            return;
        };
        let initial_progress = (time / fade.initial_duration).clamp(0.0, 1.0);
        let initial = (std::f32::consts::FRAC_PI_2 * initial_progress).cos();
        let flash_age = time - fade.flash_at;
        let flash = if (0.0..fade.flash_duration).contains(&flash_age) {
            fade.flash_alpha * (1.0 - flash_age / fade.flash_duration)
        } else {
            0.0
        };
        surface.fill(fade.color.0, initial.max(flash));
    }
}

fn draw_mask(surface: &mut Surface, bitmap: &[u8], width: usize, x: i32, y: i32, color: [u8; 4]) {
    for (index, alpha) in bitmap.iter().enumerate() {
        if *alpha == 0 {
            continue;
        }
        let px = x + (index % width) as i32;
        let py = y + (index / width) as i32;
        surface.blend(
            px,
            py,
            [color[0], color[1], color[2], *alpha],
            f32::from(color[3]) / 255.0,
        );
    }
}

fn transition(time: f32, delay: f32, duration: f32, easing: Easing) -> f32 {
    let progress = ((time - delay) / duration).clamp(0.0, 1.0);
    ease(progress, easing)
}

pub fn ease(value: f32, easing: Easing) -> f32 {
    match easing {
        Easing::Linear => value,
        Easing::Ease => 0.5 - (std::f32::consts::PI * value).cos() * 0.5,
        Easing::Cubic => {
            if value < 0.5 {
                (value * 2.0).powi(3) * 0.5
            } else {
                1.0 - ((1.0 - value) * 2.0).powi(3) * 0.5
            }
        }
        Easing::Quint => 1.0 - (1.0 - value).powi(5),
        Easing::Bounce => {
            let n = 7.5625;
            if value < 1.0 / 2.75 {
                n * value * value
            } else if value < 2.0 / 2.75 {
                let v = value - 1.5 / 2.75;
                n * v * v + 0.75
            } else if value < 2.5 / 2.75 {
                let v = value - 2.25 / 2.75;
                n * v * v + 0.9375
            } else {
                let v = value - 2.625 / 2.75;
                n * v * v + 0.984375
            }
        }
    }
}

pub fn entrance_settled(entrance: &EntranceSpec, time: f32) -> bool {
    time >= entrance.delay + entrance.duration
}
