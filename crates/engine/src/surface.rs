use crate::image::Image;

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
}
