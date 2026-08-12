use png::{ColorType, Transformations};
use std::io::Cursor;

const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Image {
    pub fn decode(label: &str, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(format!("image is too large: {label}"));
        }
        let mut decoder = png::Decoder::new_with_limits(
            Cursor::new(bytes),
            png::Limits {
                bytes: MAX_IMAGE_BYTES,
            },
        );
        decoder.set_transformations(Transformations::normalize_to_color8());
        let mut reader = decoder
            .read_info()
            .map_err(|error| format!("cannot decode {label}: {error}"))?;
        if reader.output_buffer_size() > MAX_IMAGE_BYTES {
            return Err(format!("image is too large: {label}"));
        }
        let mut source = vec![0; reader.output_buffer_size()];
        let frame = reader
            .next_frame(&mut source)
            .map_err(|error| format!("cannot decode {label}: {error}"))?;
        source.truncate(frame.buffer_size());
        let mut pixels = Vec::with_capacity(frame.width as usize * frame.height as usize * 4);
        match frame.color_type {
            ColorType::Rgba => pixels.extend_from_slice(&source),
            ColorType::Rgb => {
                for pixel in source.chunks_exact(3) {
                    pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            ColorType::GrayscaleAlpha => {
                for pixel in source.chunks_exact(2) {
                    pixels.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
                }
            }
            ColorType::Grayscale => {
                for value in source {
                    pixels.extend_from_slice(&[value, value, value, 255]);
                }
            }
            ColorType::Indexed => return Err("indexed PNG survived normalization".into()),
        }
        Ok(Self {
            width: frame.width,
            height: frame.height,
            pixels,
        })
    }

    pub fn rgba(&self, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[offset],
            self.pixels[offset + 1],
            self.pixels[offset + 2],
            self.pixels[offset + 3],
        ]
    }

    pub fn sample_bilinear(&self, x: f32, y: f32) -> [u8; 4] {
        let x = x.clamp(0.0, self.width.saturating_sub(1) as f32);
        let y = y.clamp(0.0, self.height.saturating_sub(1) as f32);
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let samples = [
            (self.rgba(x0, y0), (1.0 - fx) * (1.0 - fy)),
            (self.rgba(x1, y0), fx * (1.0 - fy)),
            (self.rgba(x0, y1), (1.0 - fx) * fy),
            (self.rgba(x1, y1), fx * fy),
        ];
        let mut result = [0; 4];
        for channel in 0..4 {
            result[channel] = samples
                .iter()
                .map(|(pixel, weight)| f32::from(pixel[channel]) * weight)
                .sum::<f32>()
                .round() as u8;
        }
        result
    }
}
