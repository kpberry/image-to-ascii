use std::f32::consts::PI;

use ::image::{DynamicImage, GenericImageView};
use image::{GenericImage, Rgba};

pub trait Image<T> {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
    fn get_pixel(&self, x: usize, y: usize) -> T;
    fn set_pixel(&mut self, x: usize, y: usize, pixel: T);

    fn get_dimensions(&self) -> (usize, usize) {
        (self.get_width(), self.get_height())
    }
}

#[derive(Clone)]
pub struct LumaImage<T> {
    width: usize,
    height: usize,
    pixels: Vec<T>,
}

impl<T: Copy> Image<T> for LumaImage<T> {
    fn get_width(&self) -> usize {
        self.width
    }

    fn get_height(&self) -> usize {
        self.height
    }

    fn get_pixel(&self, x: usize, y: usize) -> T {
        self.pixels[self.width * y + x]
    }

    fn set_pixel(&mut self, x: usize, y: usize, pixel: T) {
        self.pixels[self.width * y + x] = pixel;
    }
}

#[inline]
fn rgb_component_to_linear(component: u8) -> f32 {
    let c = (component as f32) / 255.0;
    // Magic numbers from https://en.wikipedia.org/wiki/Grayscale#Converting_color_to_grayscale.
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn rgba_to_grayscale(r: u8, g: u8, b: u8, a: u8) -> f32 {
    // Magic numbers from https://en.wikipedia.org/wiki/Grayscale#Converting_color_to_grayscale.
    let luma = 0.2126 * rgb_component_to_linear(r)
        + 0.7152 * rgb_component_to_linear(g)
        + 0.0722 * rgb_component_to_linear(b);
    let alpha = a as f32 / 255.0;
    luma * alpha
}

impl LumaImage<f32> {
    pub fn colorimetric_grayscale_from(value: &DynamicImage) -> Self {
        let lumas: Vec<f32> = value
            .pixels()
            .map(|(_, _, pixel)| rgba_to_grayscale(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect();

        LumaImage {
            width: value.width() as usize,
            height: value.height() as usize,
            pixels: lumas,
        }
    }

    pub fn naive_grayscale_from(value: &DynamicImage) -> Self {
        let lumas: Vec<f32> = value
            .pixels()
            .map(|(_, _, pixel)| {
                let sum = pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32;
                let alpha = pixel[3] as f32;
                sum * alpha / (3. * 255. * 255.)
            })
            .collect();

        LumaImage {
            width: value.width() as usize,
            height: value.height() as usize,
            pixels: lumas,
        }
    }
}

impl From<LumaImage<f32>> for DynamicImage {
    fn from(value: LumaImage<f32>) -> Self {
        let mut result = DynamicImage::new_luma8(value.width as u32, value.height as u32);
        for y in 0..value.height {
            for x in 0..value.width {
                let v = (value.get_pixel(x, y) * 255.) as u8;
                result.put_pixel(x as u32, y as u32, Rgba { 0: [v, v, v, 255] });
            }
        }
        result
    }
}

impl LumaImage<f32> {
    pub fn convolve_horizontal(&mut self, kernel: &[f32]) {
        // offset kernel to keep output size the same
        let kernel_col_offset = ((kernel.len() - 1) / 2) as isize;

        let (width, height) = (self.width as isize, self.height as isize);

        for y in 0..height {
            for x in 0..width {
                let mut total = 0.0;
                for kx in 0..(kernel.len() as isize) {
                    let dx = x + kx - kernel_col_offset;

                    if dx >= 0 && dx < width {
                        let pixel = self.get_pixel(dx as usize, y as usize);
                        total += pixel * kernel[kx as usize];
                    }
                }
                self.set_pixel(x as usize, y as usize, total);
            }
        }
    }

    pub fn convolve_vertical(&mut self, kernel: &[f32]) {
        // offset kernel to keep output size the same
        let kernel_row_offset = ((kernel.len() - 1) / 2) as isize;

        let (width, height) = (self.width as isize, self.height as isize);

        // ordering here is cache friendly (has O(len(kernel)) cache lines loaded at a time)
        for y in 0..height {
            for x in 0..width {
                let mut total = 0.0;
                for ky in 0..(kernel.len() as isize) {
                    let dy = y + ky - kernel_row_offset;

                    if dy >= 0 && dy < height {
                        let pixel = self.get_pixel(x as usize, dy as usize);
                        total += pixel * kernel[ky as usize];
                    }
                }
                self.set_pixel(x as usize, y as usize, total);
            }
        }
    }

    pub fn convolve_2d(&self, kernel: &[Vec<f32>]) -> LumaImage<f32> {
        let mut result = LumaImage {
            width: self.width,
            height: self.height,
            pixels: vec![0.; self.pixels.len()],
        };
        // offset kernel to keep output size the same
        let kernel_row_offset = ((kernel.len() - 1) / 2) as isize;
        let kernel_col_offset = ((kernel[0].len() - 1) / 2) as isize;

        let (width, height) = (self.width as isize, self.height as isize);

        for y in 0..height {
            for x in 0..width {
                let mut total = 0.0;
                for ky in 0..(kernel.len() as isize) {
                    for kx in 0..(kernel[0].len() as isize) {
                        let dy = y + ky - kernel_row_offset;
                        let dx = x + kx - kernel_col_offset;

                        if dy >= 0 && dy < height && dx >= 0 && dx < width {
                            let pixel = self.get_pixel(dx as usize, dy as usize);
                            total += pixel * kernel[ky as usize][kx as usize];
                        }
                    }
                }
                result.set_pixel(x as usize, y as usize, total);
            }
        }

        result
    }

    pub fn blur(&mut self, sigma: f32, size: isize) {
        let kernel = normalize_f32(&get_gaussian_kernel(sigma, size));
        self.convolve_vertical(&kernel);
        self.convolve_horizontal(&kernel);
    }

    pub fn detect_edges(&self) -> LumaImage<f32> {
        // TODO can this be separated?
        let kernel = [vec![0., -1., 0.], vec![-1., 4., -1.], vec![0., -1., 0.]];
        self.convolve_2d(&kernel)
    }

    pub fn resize(&self, width: usize, height: usize) -> LumaImage<f32> {
        // TODO allow passing weights here (triangular, gaussian, etc.)
        let mut result = LumaImage {
            width,
            height,
            pixels: vec![0.; width * height],
        };

        let x_ratio = self.width as f32 / width as f32;
        let y_ratio = self.height as f32 / height as f32;

        for y in 0..height {
            for x in 0..width {
                let in_x = x as f32 * x_ratio;
                let in_y = y as f32 * y_ratio;

                let ux = in_x.floor() as usize;
                let uy = in_y.floor() as usize;

                let tl = self.get_pixel(ux, uy);
                let tr = self.get_pixel((ux + 1).min(self.width - 1), uy);
                let bl = self.get_pixel(ux, (uy + 1).min(self.height - 1));
                let br =
                    self.get_pixel((ux + 1).min(self.width - 1), (uy + 1).min(self.height - 1));

                let ty = in_y - uy as f32;
                let tx = in_x - ux as f32;
                let p = lerp_f32(lerp_f32(tl, tx, tr), ty, lerp_f32(bl, tx, br));

                result.set_pixel(x, y, p);
            }
        }

        result
    }

    pub fn grid(&self) -> Vec<Vec<f32>> {
        let mut i = 0;
        (0..self.height)
            .map(|_| {
                (0..self.width)
                    .map(|_| {
                        let p = self.pixels[i];
                        i += 1;
                        p
                    })
                    .collect()
            })
            .collect()
    }

    pub fn pixels(&self) -> &Vec<f32> {
        &self.pixels
    }
}

pub fn get_gaussian_kernel(sigma: f32, size: isize) -> Vec<f32> {
    // 1/sqrt(pi * 2 * sigma^2) * e^(-x^2/(2 * sigma^2))
    let a = 2. * sigma.powi(2);
    let b = 1. / (PI * a).sqrt();
    (-size..size + 1)
        .map(|x| b * (x.pow(2) as f32 / -a).exp())
        .collect()
}

pub fn normalize_f32(v: &[f32]) -> Vec<f32> {
    let inv_total = 1. / v.iter().sum::<f32>();
    v.iter().cloned().map(|x| x * inv_total).collect()
}

pub fn lerp_f32(a: f32, t: f32, b: f32) -> f32 {
    a + t * (b - a)
}
