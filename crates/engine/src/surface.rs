use crate::image::Image;
use fontdue::{Font, FontSettings};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Rule used to decide whether a point is inside a self-intersecting path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    EvenOdd,
    Winding,
}

/// Coverage policy for filled shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntialiasMode {
    /// Four-by-four fixed samples per pixel.
    Coverage,
    /// One sample at the pixel centre.
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillOptions {
    pub rule: FillRule,
    pub antialias: AntialiasMode,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            rule: FillRule::EvenOdd,
            antialias: AntialiasMode::Coverage,
        }
    }
}

impl Surface {
    pub fn new(width: u32, height: u32, color: [u8; 4]) -> Self {
        let mut pixels = vec![0; width as usize * height as usize * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, color: [u8; 4], opacity: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let offset = ((y as u32 * self.width + x as u32) * 4) as usize;
        let alpha = (f32::from(color[3]) / 255.0 * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let inverse = 1.0 - alpha;
        for (index, channel) in color[..3].iter().enumerate() {
            self.pixels[offset + index] = (f32::from(*channel) * alpha
                + f32::from(self.pixels[offset + index]) * inverse)
                .round() as u8;
        }
        self.pixels[offset + 3] = 255;
    }

    pub fn fill(&mut self, color: [u8; 4], opacity: f32) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                self.blend(x, y, color, opacity);
            }
        }
    }

    pub(crate) fn draw_image(
        &mut self,
        image: &Image,
        left: f32,
        top: f32,
        scale: f32,
        alpha: f32,
    ) {
        if scale <= 0.0 || alpha <= 0.0 {
            return;
        }
        let target_width = (image.width as f32 * scale).round().max(1.0) as i32;
        let target_height = (image.height as f32 * scale).round().max(1.0) as i32;
        let start_x = left.floor().max(0.0) as i32;
        let start_y = top.floor().max(0.0) as i32;
        let end_x = (left + target_width as f32).ceil().min(self.width as f32) as i32;
        let end_y = (top + target_height as f32).ceil().min(self.height as f32) as i32;
        for y in start_y..end_y {
            let source_y = (y as f32 + 0.5 - top) / scale - 0.5;
            for x in start_x..end_x {
                let source_x = (x as f32 + 0.5 - left) / scale - 0.5;
                self.blend(x, y, image.sample_bilinear(source_x, source_y), alpha);
            }
        }
    }

    pub(crate) fn draw_image_region(
        &mut self,
        image: &Image,
        left: f32,
        top: f32,
        scale: f32,
        alpha: f32,
        source: [u32; 4],
    ) {
        if scale <= 0.0 || alpha <= 0.0 || source[2] == 0 || source[3] == 0 {
            return;
        }
        self.draw_image_region_scaled(
            image,
            left,
            top,
            source[2] as f32 * scale,
            source[3] as f32 * scale,
            alpha,
            source,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_region_scaled(
        &mut self,
        image: &Image,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
        alpha: f32,
        source: [u32; 4],
    ) {
        if width <= 0.0 || height <= 0.0 || alpha <= 0.0 || source[2] == 0 || source[3] == 0 {
            return;
        }
        let tw = width.round().max(1.0) as i32;
        let th = height.round().max(1.0) as i32;
        let sx = left.floor().max(0.0) as i32;
        let sy = top.floor().max(0.0) as i32;
        let ex = (left + tw as f32).ceil().min(self.width as f32) as i32;
        let ey = (top + th as f32).ceil().min(self.height as f32) as i32;
        for y in sy..ey {
            for x in sx..ex {
                let px =
                    source[0] as f32 + (x as f32 + 0.5 - left) * source[2] as f32 / width - 0.5;
                let py =
                    source[1] as f32 + (y as f32 + 0.5 - top) * source[3] as f32 / height - 0.5;
                self.blend(x, y, image.sample_bilinear(px, py), alpha);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_image_nine_slice(
        &mut self,
        image: &Image,
        left: f32,
        top: f32,
        scale: f32,
        alpha: f32,
        source: [u32; 4],
        slice: [u32; 4],
        width: f32,
        height: f32,
    ) {
        let [sx, sy, sw, sh] = source;
        let [sl, st, sr, sb] = slice;
        let dw = width * scale;
        let dh = height * scale;
        let xs = [0.0, sl as f32 * scale, dw - sr as f32 * scale, dw];
        let ys = [0.0, st as f32 * scale, dh - sb as f32 * scale, dh];
        let src_x = [sx, sx + sl, sx + sw - sr];
        let src_y = [sy, sy + st, sy + sh - sb];
        let src_w = [sl, sw - sl - sr, sr];
        let src_h = [st, sh - st - sb, sb];
        for row in 0..3 {
            for col in 0..3 {
                self.draw_image_region_scaled(
                    image,
                    left + xs[col],
                    top + ys[row],
                    xs[col + 1] - xs[col],
                    ys[row + 1] - ys[row],
                    alpha,
                    [src_x[col], src_y[row], src_w[col], src_h[row]],
                );
            }
        }
    }

    pub fn encode_png(&self) -> Result<Vec<u8>, String> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut output, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|error| format!("cannot encode PNG: {error}"))?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|error| format!("cannot encode PNG: {error}"))?;
        }
        Ok(output)
    }

    pub fn packed_rgb(&self) -> Vec<u32> {
        self.pixels
            .chunks_exact(4)
            .map(|pixel| {
                (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2])
            })
            .collect()
    }
}

fn inside_polygon(points: &[[f32; 2]], px: f32, py: f32, rule: FillRule) -> bool {
    let mut winding = 0i32;
    let mut parity = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, yi) = (points[i][0], points[i][1]);
        let (xj, yj) = (points[j][0], points[j][1]);
        if (yi > py) != (yj > py) {
            let cross = (xj - xi) * (py - yi) / (yj - yi) + xi;
            if px < cross {
                parity = !parity;
                winding += if yj > yi { 1 } else { -1 };
            }
        }
        j = i;
    }
    match rule {
        FillRule::EvenOdd => parity,
        FillRule::Winding => winding != 0,
    }
}

/// How an image is fitted into a destination rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFit {
    Contain,
    Cover,
    Stretch,
}

/// A decoded font that can be reused by a [`Canvas`].
#[derive(Clone)]
pub struct FontFace(Font);

impl FontFace {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        Font::from_bytes(bytes, FontSettings::default())
            .map(Self)
            .map_err(|e| format!("cannot decode font: {e}"))
    }

    pub fn measure(&self, text: &str, size: f32, letter_spacing: f32) -> [f32; 2] {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for (index, character) in text.chars().enumerate() {
            let metrics = self.0.metrics(character, size);
            if index > 0 {
                width += letter_spacing;
            }
            width += metrics.advance_width;
            height = height.max(metrics.height as f32);
        }
        [width, height]
    }
}

/// Small immediate-mode drawing facade for hosts and product crates.
/// Coordinates are design-space pixels and output is deterministic RGBA8.
pub struct Canvas {
    surface: Surface,
    logical_width: u32,
    logical_height: u32,
    density: f32,
}

impl Canvas {
    pub fn new(width: u32, height: u32, clear: [u8; 4]) -> Self {
        Self {
            surface: Surface::new(width, height, clear),
            logical_width: width,
            logical_height: height,
            density: 1.0,
        }
    }
    /// Create a high-density surface while keeping all drawing coordinates in
    /// logical pixels. Hosts can present the returned physical pixels directly
    /// on a Retina/HiDPI backing surface without post-raster scaling text.
    pub fn new_scaled(width: u32, height: u32, density: f32, clear: [u8; 4]) -> Self {
        let density = if density.is_finite() {
            density.clamp(1.0, 4.0)
        } else {
            1.0
        };
        let physical_width = (width as f32 * density).round().max(1.0) as u32;
        let physical_height = (height as f32 * density).round().max(1.0) as u32;
        Self {
            surface: Surface::new(physical_width, physical_height, clear),
            logical_width: width,
            logical_height: height,
            density,
        }
    }
    pub fn width(&self) -> u32 {
        self.logical_width
    }
    pub fn height(&self) -> u32 {
        self.logical_height
    }
    pub fn density(&self) -> f32 {
        self.density
    }
    pub fn clear(&mut self, color: [u8; 4]) {
        self.surface = Surface::new(self.surface.width, self.surface.height, color);
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn into_surface(self) -> Surface {
        self.surface
    }
    /// Copies an opaque surface at exact physical backing-pixel coordinates.
    /// Returns `false` when the source contains non-opaque pixels.
    pub fn blit_surface(&mut self, source: &Surface, left: i32, top: i32) -> bool {
        if source.pixels.chunks_exact(4).any(|pixel| pixel[3] != 255) {
            return false;
        }
        let source_x = (-left).max(0).min(source.width as i32);
        let source_y = (-top).max(0).min(source.height as i32);
        let source_end_x = (self.surface.width as i32 - left)
            .max(0)
            .min(source.width as i32);
        let source_end_y = (self.surface.height as i32 - top)
            .max(0)
            .min(source.height as i32);
        if source_x >= source_end_x || source_y >= source_end_y {
            return true;
        }
        let copy_bytes = (source_end_x - source_x) as usize * 4;
        for sy in source_y..source_end_y {
            let source_offset = (sy as usize * source.width as usize + source_x as usize) * 4;
            let destination_offset = ((top + sy) as usize * self.surface.width as usize
                + (left + source_x) as usize)
                * 4;
            self.surface.pixels[destination_offset..destination_offset + copy_bytes]
                .copy_from_slice(&source.pixels[source_offset..source_offset + copy_bytes]);
        }
        true
    }
    pub fn fill_rect(&mut self, rect: [f32; 4], color: [u8; 4]) {
        self.rect(rect, color, None);
    }
    /// Draw an evenly spaced dot field inside a rectangular clip without
    /// expanding each dot into a full-canvas rectangle pass.
    pub fn dot_grid(&mut self, [x, y, w, h]: [f32; 4], spacing: u32, radius: u32, color: [u8; 4]) {
        if spacing == 0 || radius == 0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let scale = self.density;
        let spacing = (spacing as f32 * scale).round().max(1.0) as usize;
        let radius = (radius as f32 * scale).round().max(1.0) as i32;
        let start_x = (x * scale).floor().max(0.0) as i32;
        let start_y = (y * scale).floor().max(0.0) as i32;
        let end_x = ((x + w) * scale).ceil().min(self.surface.width as f32) as i32;
        let end_y = ((y + h) * scale).ceil().min(self.surface.height as f32) as i32;
        for center_y in (start_y..end_y).step_by(spacing) {
            for center_x in (start_x..end_x).step_by(spacing) {
                for offset_y in -radius..=radius {
                    for offset_x in -radius..=radius {
                        if offset_x * offset_x + offset_y * offset_y <= radius * radius {
                            self.surface.blend(
                                center_x + offset_x,
                                center_y + offset_y,
                                color,
                                1.0,
                            );
                        }
                    }
                }
            }
        }
    }
    pub fn stroke_rect(&mut self, rect: [f32; 4], color: [u8; 4], width: f32) {
        self.rect(rect, color, Some(width));
    }
    fn rect(&mut self, [x, y, w, h]: [f32; 4], color: [u8; 4], stroke: Option<f32>) {
        let scale = self.density;
        let (x, y, w, h) = (x * scale, y * scale, w * scale, h * scale);
        let edge = stroke.map(|value| value * scale).unwrap_or(h.max(w));
        let start_x = x.floor().max(0.0) as i32;
        let start_y = y.floor().max(0.0) as i32;
        let end_x = (x + w).ceil().min(self.surface.width as f32) as i32;
        let end_y = (y + h).ceil().min(self.surface.height as f32) as i32;
        for py in start_y..end_y {
            for px in start_x..end_x {
                let inside = (px as f32) >= x
                    && (px as f32) < x + w
                    && (py as f32) >= y
                    && (py as f32) < y + h;
                let border = stroke.is_some()
                    && ((px as f32) < x + edge
                        || (px as f32) >= x + w - edge
                        || (py as f32) < y + edge
                        || (py as f32) >= y + h - edge);
                if inside && (stroke.is_none() || border) {
                    self.surface.blend(px, py, color, 1.0);
                }
            }
        }
    }
    pub fn rounded_rect(&mut self, [x, y, w, h]: [f32; 4], radius: f32, color: [u8; 4]) {
        let scale = self.density;
        let (x, y, w, h, radius) = (x * scale, y * scale, w * scale, h * scale, radius * scale);
        let r = radius.max(0.0).min(w.min(h) / 2.0);
        let start_x = x.floor().max(0.0) as i32;
        let start_y = y.floor().max(0.0) as i32;
        let end_x = (x + w).ceil().min(self.surface.width as f32) as i32;
        let end_y = (y + h).ceil().min(self.surface.height as f32) as i32;
        for py in start_y..end_y {
            for px in start_x..end_x {
                let qx = (px as f32 - (x + r).max(x).min(x + w - r)).abs();
                let qy = (py as f32 - (y + r).max(y).min(y + h - r)).abs();
                if qx * qx + qy * qy <= r * r
                    || ((px as f32) >= x + r && (px as f32) < x + w - r)
                    || ((py as f32) >= y + r && (py as f32) < y + h - r)
                {
                    self.surface.blend(px, py, color, 1.0);
                }
            }
        }
    }
    pub fn polygon(&mut self, points: &[[f32; 2]], color: [u8; 4]) {
        self.polygon_with_options(points, color, FillOptions::default());
    }

    /// Fills a polygon using a clipped, deterministic coverage mask.
    pub fn polygon_with_options(
        &mut self,
        points: &[[f32; 2]],
        color: [u8; 4],
        options: FillOptions,
    ) {
        if points.len() < 3 {
            return;
        }
        let points = points
            .iter()
            .map(|point| [point[0] * self.density, point[1] * self.density])
            .collect::<Vec<_>>();
        if points
            .iter()
            .any(|p| !p[0].is_finite() || !p[1].is_finite())
        {
            return;
        }
        let min_x = points
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as i32;
        let max_x = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(self.surface.width as f32) as i32;
        let min_y = points
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as i32;
        let max_y = points
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(self.surface.height as f32) as i32;
        for y in min_y..max_y {
            if options.antialias == AntialiasMode::Coverage {
                let mut coverage = vec![0u8; (max_x - min_x) as usize];
                for sy in 0..4 {
                    let scan_y = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let mut crossings = Vec::with_capacity(points.len());
                    let mut j = points.len() - 1;
                    for i in 0..points.len() {
                        let (xi, yi) = (points[i][0], points[i][1]);
                        let (xj, yj) = (points[j][0], points[j][1]);
                        if (yi > scan_y) != (yj > scan_y) {
                            let x = xi + (scan_y - yi) * (xj - xi) / (yj - yi);
                            crossings.push((x, if yj > yi { 1i32 } else { -1i32 }));
                        }
                        j = i;
                    }
                    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let mut intervals = Vec::new();
                    match options.rule {
                        FillRule::EvenOdd => {
                            for pair in crossings.chunks_exact(2) {
                                intervals.push((pair[0].0, pair[1].0));
                            }
                        }
                        FillRule::Winding => {
                            let mut winding = 0;
                            let mut start = 0.0;
                            for (x, delta) in crossings {
                                let old = winding;
                                winding += delta;
                                if old == 0 && winding != 0 {
                                    start = x;
                                }
                                if old != 0 && winding == 0 {
                                    intervals.push((start, x));
                                }
                            }
                        }
                    }
                    for (left, right) in intervals {
                        let first = left.floor() as i32;
                        let last = right.ceil() as i32;
                        for px in first.max(min_x)..last.min(max_x) {
                            let mut hits = 0;
                            for sx in 0..4 {
                                let sample_x = px as f32 + (sx as f32 + 0.5) / 4.0;
                                if sample_x >= left && sample_x < right {
                                    hits += 1;
                                }
                            }
                            coverage[(px - min_x) as usize] += hits;
                        }
                    }
                }
                for (index, hits) in coverage.into_iter().enumerate() {
                    if hits != 0 {
                        self.surface
                            .blend(min_x + index as i32, y, color, hits as f32 / 16.0);
                    }
                }
                continue;
            }
            for x in min_x..max_x {
                if inside_polygon(&points, x as f32 + 0.5, y as f32 + 0.5, options.rule) {
                    self.surface.blend(x, y, color, 1.0);
                }
            }
        }
    }
    pub fn image(&mut self, image: &Image, rect: [f32; 4], fit: ImageFit, opacity: f32) {
        let (x, y, w, h) = (
            rect[0] * self.density,
            rect[1] * self.density,
            rect[2] * self.density,
            rect[3] * self.density,
        );
        if fit == ImageFit::Stretch {
            self.surface.draw_image_region_scaled(
                image,
                x,
                y,
                w,
                h,
                opacity,
                [0, 0, image.width, image.height],
            );
            return;
        }
        let scale = match fit {
            ImageFit::Contain => (w / image.width as f32).min(h / image.height as f32),
            ImageFit::Cover => (w / image.width as f32).max(h / image.height as f32),
            ImageFit::Stretch => unreachable!(),
        };
        let dw = image.width as f32 * scale;
        let dh = image.height as f32 * scale;
        let left = x + (w - dw) / 2.0;
        let top = y + (h - dh) / 2.0;
        self.surface.draw_image(image, left, top, scale, opacity);
    }
    pub fn text(
        &mut self,
        face: &FontFace,
        text: &str,
        origin: [f32; 2],
        size: f32,
        color: [u8; 4],
        outline: Option<(f32, [u8; 4])>,
    ) {
        self.text_spaced(face, text, origin, size, color, outline, 0.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_spaced(
        &mut self,
        face: &FontFace,
        text: &str,
        origin: [f32; 2],
        size: f32,
        color: [u8; 4],
        outline: Option<(f32, [u8; 4])>,
        letter_spacing: f32,
    ) {
        let density = self.density;
        let mut x = origin[0] * density;
        let baseline = origin[1] * density;
        let size = size * density;
        let letter_spacing = letter_spacing * density;
        for (index, ch) in text.chars().enumerate() {
            if index > 0 {
                x += letter_spacing;
            }
            let (metrics, bitmap) = face.0.rasterize(ch, size);
            let glyph_x = x + metrics.xmin as f32;
            let glyph_y = baseline - metrics.ymin as f32 - metrics.height as f32;
            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let a = bitmap[gy * metrics.width + gx];
                    if a > 0 {
                        if let Some((radius, oc)) = outline {
                            let radius = radius * density;
                            for oy in -(radius as i32)..=radius as i32 {
                                for ox in -(radius as i32)..=radius as i32 {
                                    self.surface.blend(
                                        glyph_x as i32 + gx as i32 + ox,
                                        glyph_y as i32 + gy as i32 + oy,
                                        oc,
                                        f32::from(a) / 255.0,
                                    );
                                }
                            }
                        }
                        self.surface.blend(
                            glyph_x as i32 + gx as i32,
                            glyph_y as i32 + gy as i32,
                            color,
                            f32::from(a) / 255.0,
                        );
                    }
                }
            }
            x += metrics.advance_width;
        }
    }
    pub fn encode_png(&self) -> Result<Vec<u8>, String> {
        self.surface.encode_png()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, pixels: Vec<u8>) -> Image {
        Image {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn region_scaling_supports_non_uniform_destination_geometry() {
        let source = image(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        );
        let mut surface = Surface::new(4, 2, [0, 0, 0, 255]);
        surface.draw_image_region_scaled(&source, 0.0, 0.0, 4.0, 2.0, 1.0, [0, 0, 2, 2]);
        assert!(surface.pixels[0] > surface.pixels[1]);
        let right = ((3 * 4) + 1) as usize;
        assert!(surface.pixels[right] > surface.pixels[right - 1]);
    }

    #[test]
    fn canvas_stretch_fills_the_requested_destination() {
        let source = image(1, 1, vec![220, 10, 30, 255]);
        let mut canvas = Canvas::new(5, 4, [0, 0, 0, 255]);
        canvas.image(&source, [1.0, 1.0, 3.0, 2.0], ImageFit::Stretch, 1.0);
        assert_eq!(canvas.surface().pixels[24..28], [220, 10, 30, 255]);
        assert_eq!(canvas.surface().pixels[52..56], [220, 10, 30, 255]);
        assert_eq!(canvas.surface().pixels[0..4], [0, 0, 0, 255]);
    }

    #[test]
    fn scaled_canvas_preserves_logical_geometry_at_physical_density() {
        let mut canvas = Canvas::new_scaled(8, 5, 2.0, [0, 0, 0, 255]);
        assert_eq!([canvas.width(), canvas.height()], [8, 5]);
        assert_eq!([canvas.surface().width, canvas.surface().height], [16, 10]);
        assert_eq!(canvas.density(), 2.0);
        canvas.fill_rect([1.0, 1.0, 2.0, 1.0], [255, 0, 0, 255]);
        let at = |x: usize, y: usize| (y * 16 + x) * 4;
        assert_eq!(
            canvas.surface().pixels[at(2, 2)..at(2, 2) + 4],
            [255, 0, 0, 255]
        );
        assert_eq!(
            canvas.surface().pixels[at(5, 3)..at(5, 3) + 4],
            [255, 0, 0, 255]
        );
        assert_eq!(
            canvas.surface().pixels[at(6, 3)..at(6, 3) + 4],
            [0, 0, 0, 255]
        );
    }

    #[test]
    fn dot_grid_is_clipped_and_deterministic() {
        let mut canvas = Canvas::new(12, 10, [0, 0, 0, 255]);
        canvas.dot_grid([2.0, 2.0, 7.0, 5.0], 4, 1, [255, 255, 255, 128]);
        let first = canvas.encode_png().unwrap();
        let mut replay = Canvas::new(12, 10, [0, 0, 0, 255]);
        replay.dot_grid([2.0, 2.0, 7.0, 5.0], 4, 1, [255, 255, 255, 128]);
        assert_eq!(first, replay.encode_png().unwrap());
        assert_eq!(canvas.surface().pixels[0..4], [0, 0, 0, 255]);
        assert_ne!(
            canvas.surface().pixels[((2 * 12 + 2) * 4)..((2 * 12 + 2) * 4 + 4)],
            [0, 0, 0, 255]
        );
    }

    #[test]
    fn nine_slice_preserves_corner_colors_while_expanding_center() {
        let source = image(
            3,
            3,
            vec![
                255, 0, 0, 255, 20, 20, 20, 255, 0, 255, 0, 255, 20, 20, 20, 255, 40, 40, 40, 255,
                20, 20, 20, 255, 0, 0, 255, 255, 20, 20, 20, 255, 255, 255, 0, 255,
            ],
        );
        let mut surface = Surface::new(7, 5, [0, 0, 0, 255]);
        surface.draw_image_nine_slice(
            &source,
            0.0,
            0.0,
            1.0,
            1.0,
            [0, 0, 3, 3],
            [1, 1, 1, 1],
            7.0,
            5.0,
        );
        assert_eq!(&surface.pixels[0..3], &[255, 0, 0]);
        let bottom_right = ((4 * 7 + 6) * 4) as usize;
        assert_eq!(
            &surface.pixels[bottom_right..bottom_right + 3],
            &[255, 255, 0]
        );
        let center = ((2 * 7 + 3) * 4) as usize;
        assert_eq!(&surface.pixels[center..center + 3], &[40, 40, 40]);
    }

    #[test]
    fn canvas_primitives_and_png_are_deterministic() {
        let mut canvas = Canvas::new(16, 12, [0, 0, 0, 255]);
        canvas.fill_rect([1.0, 1.0, 4.0, 3.0], [255, 0, 0, 255]);
        canvas.polygon(&[[6.0, 1.0], [12.0, 1.0], [9.0, 6.0]], [0, 255, 0, 255]);
        let first = canvas.encode_png().expect("png");
        let second = canvas.encode_png().expect("png");
        assert_eq!(first, second);
        assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn polygon_coverage_has_fractional_diagonal_and_is_replayable() {
        let points = [[0.2, 0.2], [6.7, 1.1], [1.0, 6.8]];
        let mut a = Canvas::new(8, 8, [0, 0, 0, 255]);
        a.polygon(&points, [255, 255, 255, 128]);
        let mut b = Canvas::new(8, 8, [0, 0, 0, 255]);
        b.polygon(&points, [255, 255, 255, 128]);
        assert_eq!(a.surface().pixels, b.surface().pixels);
        assert!(a
            .surface()
            .pixels
            .chunks_exact(4)
            .any(|p| p[0] > 0 && p[0] < 128));
    }

    #[test]
    fn polygon_rejects_nonfinite_and_degenerate_input() {
        let mut canvas = Canvas::new(4, 4, [3, 4, 5, 255]);
        canvas.polygon(&[[0.0, 0.0], [f32::NAN, 1.0], [2.0, 2.0]], [255, 0, 0, 255]);
        canvas.polygon_with_options(
            &[[1.0, 1.0], [1.0, 1.0], [1.0, 1.0]],
            [255, 0, 0, 255],
            FillOptions {
                rule: FillRule::Winding,
                antialias: AntialiasMode::None,
            },
        );
        assert!(canvas
            .surface()
            .pixels
            .chunks_exact(4)
            .all(|p| p == [3, 4, 5, 255]));
    }

    #[test]
    fn polygon_coverage_classifies_far_interior_and_exterior_exactly() {
        let mut canvas = Canvas::new(12, 12, [7, 8, 9, 255]);
        canvas.polygon(
            &[[1.0, 1.0], [11.0, 1.0], [11.0, 11.0], [1.0, 11.0]],
            [200, 0, 0, 255],
        );
        let pixel = |x: usize, y: usize| &canvas.surface().pixels[(y * 12 + x) * 4..][..4];
        assert_eq!(pixel(6, 6), [200, 0, 0, 255]);
        assert_eq!(pixel(0, 0), [7, 8, 9, 255]);
    }

    #[test]
    fn polygon_even_odd_is_vertex_order_invariant_for_concave_shapes() {
        let points = [
            [1.2, 1.2],
            [9.4, 1.2],
            [9.4, 4.3],
            [5.1, 4.3],
            [5.1, 9.1],
            [1.2, 9.1],
        ];
        let mut reversed = points;
        reversed.reverse();
        let mut forward = Canvas::new(12, 12, [10, 20, 30, 255]);
        let mut backward = Canvas::new(12, 12, [10, 20, 30, 255]);
        forward.polygon(&points, [240, 230, 220, 255]);
        backward.polygon(&reversed, [240, 230, 220, 255]);
        assert_eq!(forward.surface().pixels, backward.surface().pixels);
    }

    #[test]
    fn polygon_fill_rules_distinguish_a_twice_traced_shape() {
        let twice_traced = [
            [1.0, 1.0],
            [7.0, 1.0],
            [7.0, 7.0],
            [1.0, 7.0],
            [1.0, 1.0],
            [7.0, 1.0],
            [7.0, 7.0],
            [1.0, 7.0],
        ];
        let mut even_odd = Canvas::new(9, 9, [0, 0, 0, 255]);
        let mut winding = Canvas::new(9, 9, [0, 0, 0, 255]);
        even_odd.polygon_with_options(&twice_traced, [255, 255, 255, 255], FillOptions::default());
        winding.polygon_with_options(
            &twice_traced,
            [255, 255, 255, 255],
            FillOptions {
                rule: FillRule::Winding,
                ..FillOptions::default()
            },
        );
        let center = (4 * 9 + 4) * 4;
        assert_eq!(
            &even_odd.surface().pixels[center..center + 4],
            [0, 0, 0, 255]
        );
        assert_eq!(
            &winding.surface().pixels[center..center + 4],
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn polygon_binary_mode_has_no_fractional_pixels() {
        let mut canvas = Canvas::new(8, 8, [0, 0, 0, 255]);
        canvas.polygon_with_options(
            &[[0.2, 0.2], [6.7, 1.1], [1.0, 6.8]],
            [255, 255, 255, 255],
            FillOptions {
                antialias: AntialiasMode::None,
                ..FillOptions::default()
            },
        );
        assert!(canvas
            .surface()
            .pixels
            .chunks_exact(4)
            .all(|pixel| pixel[0] == 0 || pixel[0] == 255));
    }

    #[test]
    fn polygon_density_changes_detail_not_logical_bounds() {
        let points = [[1.25, 1.25], [6.75, 1.25], [6.75, 6.75], [1.25, 6.75]];
        let mut one = Canvas::new_scaled(8, 8, 1.0, [0, 0, 0, 255]);
        let mut two = Canvas::new_scaled(8, 8, 2.0, [0, 0, 0, 255]);
        one.polygon(&points, [255, 255, 255, 255]);
        two.polygon(&points, [255, 255, 255, 255]);
        assert_eq!((one.width(), one.height()), (two.width(), two.height()));
        assert_eq!((one.surface().width, one.surface().height), (8, 8));
        assert_eq!((two.surface().width, two.surface().height), (16, 16));
        assert_eq!(&two.surface().pixels[..4], [0, 0, 0, 255]);
        let interior = ((8 * 16 + 8) * 4) as usize;
        assert_eq!(
            &two.surface().pixels[interior..interior + 4],
            [255, 255, 255, 255]
        );
    }

    #[test]
    #[ignore = "release timing evidence; run with --ignored --release"]
    fn polygon_game_term_release_timing_evidence() {
        let mut canvas = Canvas::new_scaled(1600, 900, 2.0, [0, 0, 0, 255]);
        let polygon = [
            [80.0, 80.0],
            [1520.0, 110.0],
            [1450.0, 820.0],
            [100.0, 780.0],
        ];
        let start = std::time::Instant::now();
        for _ in 0..10 {
            canvas.polygon(&polygon, [255, 255, 255, 255]);
        }
        eprintln!(
            "10 representative 1600x900@2x polygon draws: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn opaque_surface_blit_copies_clips_and_replays() {
        let source = Surface::new(3, 2, [0, 0, 0, 255]);
        let mut canvas = Canvas::new(4, 3, [1, 2, 3, 255]);
        assert!(canvas.blit_surface(&source, -1, 1));
        assert_eq!(&canvas.surface().pixels[16..][..4], [0, 0, 0, 255]);
        let mut replay = Canvas::new(4, 3, [1, 2, 3, 255]);
        assert!(replay.blit_surface(&source, -1, 1));
        assert_eq!(canvas.surface().pixels, replay.surface().pixels);
        let translucent = Surface::new(1, 1, [9, 8, 7, 128]);
        assert!(!canvas.blit_surface(&translucent, 0, 0));
    }

    #[test]
    fn surface_blit_uses_exact_backing_coordinates_at_density() {
        let source = Surface::new(2, 1, [9, 8, 7, 255]);
        let mut canvas = Canvas::new_scaled(2, 1, 2.0, [0, 0, 0, 255]);
        assert!(canvas.blit_surface(&source, 2, 1));
        assert_eq!(&canvas.surface().pixels[24..][..4], [9, 8, 7, 255]);
    }
}
