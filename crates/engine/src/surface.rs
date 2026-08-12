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
