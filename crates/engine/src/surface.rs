use crate::image::Image;
use fontdue::{Font, FontSettings};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
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
}

impl Canvas {
    pub fn new(width: u32, height: u32, clear: [u8; 4]) -> Self {
        Self {
            surface: Surface::new(width, height, clear),
        }
    }
    pub fn width(&self) -> u32 {
        self.surface.width
    }
    pub fn height(&self) -> u32 {
        self.surface.height
    }
    pub fn clear(&mut self, color: [u8; 4]) {
        self.surface = Surface::new(self.width(), self.height(), color);
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn into_surface(self) -> Surface {
        self.surface
    }
    pub fn fill_rect(&mut self, rect: [f32; 4], color: [u8; 4]) {
        self.rect(rect, color, None);
    }
    pub fn stroke_rect(&mut self, rect: [f32; 4], color: [u8; 4], width: f32) {
        self.rect(rect, color, Some(width));
    }
    fn rect(&mut self, [x, y, w, h]: [f32; 4], color: [u8; 4], stroke: Option<f32>) {
        let edge = stroke.unwrap_or(h.max(w));
        for py in 0..self.height() as i32 {
            for px in 0..self.width() as i32 {
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
        let r = radius.max(0.0).min(w.min(h) / 2.0);
        for py in 0..self.height() as i32 {
            for px in 0..self.width() as i32 {
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
        if points.len() < 3 {
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
            .min(self.width() as f32) as i32;
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
            .min(self.height() as f32) as i32;
        for y in min_y..max_y {
            for x in min_x..max_x {
                let mut hit = false;
                let mut j = points.len() - 1;
                for i in 0..points.len() {
                    let (xi, yi) = (points[i][0], points[i][1]);
                    let (xj, yj) = (points[j][0], points[j][1]);
                    if ((yi > y as f32) != (yj > y as f32))
                        && ((x as f32) < (xj - xi) * (y as f32 - yi) / (yj - yi) + xi)
                    {
                        hit = !hit;
                    }
                    j = i;
                }
                if hit {
                    self.surface.blend(x, y, color, 1.0);
                }
            }
        }
    }
    pub fn image(&mut self, image: &Image, rect: [f32; 4], fit: ImageFit, opacity: f32) {
        let (x, y, w, h) = (rect[0], rect[1], rect[2], rect[3]);
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
        let mut x = origin[0];
        for (index, ch) in text.chars().enumerate() {
            if index > 0 {
                x += letter_spacing;
            }
            let (metrics, bitmap) = face.0.rasterize(ch, size);
            let glyph_x = x + metrics.xmin as f32;
            let glyph_y = origin[1] - metrics.ymin as f32 - metrics.height as f32;
            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let a = bitmap[gy * metrics.width + gx];
                    if a > 0 {
                        if let Some((radius, oc)) = outline {
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
}
