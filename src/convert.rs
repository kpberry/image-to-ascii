use colored::Colorize;
use std::cmp::Ordering;

use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, Luma, Rgb, RgbImage, Rgba};

use crate::font::Font;
use crate::metrics::{
    avg_color_score, denoised_jaccard_score, dot_score, jaccard_score, movement_toward_clear, occlusion_score, Metric
};

use crate::image::{Image, LumaImage};

pub type Converter = fn(&Font, &[f32]) -> char;
pub enum ConversionAlgorithm {
    Base,
    Edge,
    EdgeAugmented,
    TwoPass,
}

pub fn score_convert(score_fn: Metric, font: &Font, chunk: &[f32]) -> char {
    let max_index = font
        .chars
        .iter()
        .map(|c| score_fn(&chunk, &c.bitmap))
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        .unwrap()
        .0;
    font.chars[max_index].value
}

pub fn dot_convert(font: &Font, chunk: &[f32]) -> char {
    score_convert(dot_score, font, chunk)
}

pub fn jaccard_convert(font: &Font, chunk: &[f32]) -> char {
    score_convert(jaccard_score, font, chunk)
}

pub fn intensity_jaccard_convert(font: &Font, chunk: &[f32]) -> char {
    let max_index = font
        .chars
        .iter()
        .map(|c| denoised_jaccard_score(&chunk, &c.bitmap) - (&chunk.iter().sum() - c.intensity).abs())
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        .unwrap()
        .0;
    font.chars[max_index].value
}

pub fn occlusion_convert(font: &Font, chunk: &[f32]) -> char {
    score_convert(occlusion_score, font, chunk)
}

pub fn color_convert(font: &Font, chunk: &[f32]) -> char {
    score_convert(avg_color_score, font, chunk)
}

pub fn clear_convert(font: &Font, chunk: &[f32]) -> char {
    score_convert(movement_toward_clear, font, chunk)
}

pub fn intensity_convert(font: &Font, chunk: &[f32]) -> char {
    let intensity = chunk.iter().sum::<f32>();
    let index = intensity as usize;
    font.intensity_chars[index].value
}

fn chunk_direction(chunk: &[f32], width: usize, height: usize) -> (f32, f32) {
    let mut x_grad = 0.0;
    let mut y_grad = 0.0;
    for i in 0..height {
        for j in 0..width - 1 {
            x_grad += chunk[i * width + 1 + j] - chunk[i * width + j];
        }
    }
    for i in 0..height - 1 {
        for j in 0..width {
            y_grad += chunk[(i + 1) * width + j] - chunk[i * width + j];
        }
    }

    // contour lines (directions) are perpendicular to the gradient
    (-y_grad, x_grad)
}

pub fn direction_and_intensity_convert(font: &Font, chunk: &[f32]) -> char {
    let max_direction = (font.width * font.height * 4) as f32; // direction should never be bigger than this
    let (x_dir, y_dir) = chunk_direction(chunk, font.width, font.height);
    let intensity = chunk.iter().sum::<f32>();

    let scores: Vec<f32> = font
        .chars
        .iter()
        .map(|c| {
            let grad = -((x_dir - c.direction.0).powi(2) + (y_dir - c.direction.1).powi(2)).sqrt();
            (max_direction - grad) / (1. + (intensity - c.intensity).powf(2.))
        })
        .collect();

    font.chars
        .iter()
        .zip(scores)
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0
        .value
}

pub fn direction_convert(font: &Font, chunk: &[f32]) -> char {
    let (x_dir, y_dir) = chunk_direction(chunk, font.width, font.height);

    let scores: Vec<f32> = font
        .chars
        .iter()
        .map(|c| -((x_dir - c.direction.0).powi(2) + (y_dir - c.direction.1).powi(2)).sqrt())
        .collect();

    font.chars
        .iter()
        .zip(scores)
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0
        .value
}

pub fn get_converter(metric: &str) -> Converter {
    match &metric[..] {
        "dot" => dot_convert,
        "jaccard" => jaccard_convert,
        "occlusion" => occlusion_convert,
        "color" => color_convert,
        "clear" => clear_convert,
        "fast" | "intensity" => intensity_convert,
        "grad" | "direction-and-intensity" => direction_and_intensity_convert,
        "direction" => direction_convert,
        "intensity-jaccard" => intensity_jaccard_convert,
        _ => panic!("Unsupported metric {}", metric),
    }
}

pub fn get_conversion_algorithm(algorithm: &str) -> ConversionAlgorithm {
    match &algorithm[..] {
        "base" => ConversionAlgorithm::Base,
        "edge" => ConversionAlgorithm::Edge,
        "edge-augmented" => ConversionAlgorithm::EdgeAugmented,
        "two-pass" => ConversionAlgorithm::TwoPass,
        _ => panic!("Unsupported conversion algorithm {}", algorithm),
    }
}

fn pixels_to_chunks(
    pixels: &[f32],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
) -> Vec<Vec<f32>> {
    let chunk_size = chunk_width * chunk_height;
    let vertical_chunks = height / chunk_height;
    let horizontal_chunks = width / chunk_width;

    // not the simplest way of doing this, but should give good cache performance and avoids
    // multiplications/divisions
    let mut chunks: Vec<Vec<f32>> = Vec::with_capacity(vertical_chunks * horizontal_chunks);
    let mut y_offset = 0;
    let mut x_offset = 0;
    for _ in 0..vertical_chunks {
        let mut chunk_row: Vec<Vec<f32>> = (0..horizontal_chunks)
            .map(|_| Vec::with_capacity(chunk_size))
            .collect();

        for _ in 0..chunk_height {
            for x in 0..horizontal_chunks {
                let start = y_offset + x_offset;
                let end = start + chunk_width;
                let chunk_pixel_row = &pixels[start..end];
                chunk_row[x].extend(chunk_pixel_row);
                x_offset += chunk_width;
            }
            y_offset += width;
            x_offset = 0;
        }

        chunks.extend(chunk_row);
    }

    chunks
}

pub fn pixels_to_chars(
    pixels: &[f32],
    width: usize,
    height: usize,
    font: &Font,
    convert: Converter,
) -> Vec<char> {
    let chunks = pixels_to_chunks(pixels, width, height, font.width, font.height);
    chunks.iter().map(|chunk| convert(&font, chunk)).collect()
}

fn round_up_to_multiple(x: i32, m: i32) -> i32 {
    x + (-x % m)
}

pub fn img_to_char_rows(
    font: &Font,
    img: &LumaImage<f32>,
    convert: Converter,
    out_width: Option<usize>,
    brightness_offset: f32,
    algorithm: &ConversionAlgorithm,
) -> Vec<Vec<char>> {
    let (width, height) = img.get_dimensions();

    let out_width = if let Some(out_width) = out_width {
        out_width
    } else {
        round_up_to_multiple(width as i32, font.width as i32) as usize / font.width
    };

    let out_height = (height as f64
        * (out_width as f64 / width as f64)
        * (font.width as f64 / font.height as f64))
        .round() as usize;

    let (out_img_width, out_img_height) = (out_width * font.width, out_height * font.height);
    let resized_image = img.resize(out_img_width, out_img_height);

    let chars: Vec<char> = match algorithm {
        ConversionAlgorithm::Base => {
            let pixels: Vec<f32> = resized_image
                .pixels()
                .iter()
                .map(|y| y - brightness_offset)
                .collect();

            pixels_to_chars(&pixels, out_img_width, out_img_height, &font, convert)
        }
        ConversionAlgorithm::Edge => {
            let mut edge_img = img.clone();
            edge_img.blur(1.0, 2);
            edge_img = edge_img.detect_edges();
            edge_img = edge_img.resize(out_img_width, out_img_height);

            let pixels: Vec<f32> = edge_img
                .pixels()
                .iter()
                .map(|y| y - brightness_offset)
                .collect();

            pixels_to_chars(
                &pixels,
                out_img_width,
                out_img_height,
                &font,
                direction_convert,
            )
        }
        ConversionAlgorithm::EdgeAugmented => {
            let mut edge_img = img.clone();
            edge_img.blur(1.0, 2);
            edge_img = edge_img.detect_edges();
            edge_img = edge_img.resize(out_img_width, out_img_height);

            let pixels: Vec<f32> = resized_image
                .pixels()
                .iter()
                .zip(edge_img.pixels())
                .map(|(a, b)| a / 4. + b - brightness_offset)
                .collect();

            pixels_to_chars(&pixels, out_img_width, out_img_height, &font, convert)
        }
        ConversionAlgorithm::TwoPass => {
            let luma_pixels: Vec<f32> = resized_image
                .pixels()
                .iter()
                .map(|y| y - brightness_offset)
                .collect();

            let mut edge_img = img.clone();
            edge_img.blur(1.0, 2);
            edge_img = edge_img.detect_edges();
            edge_img = edge_img.resize(out_img_width, out_img_height);
            let edge_pixels: Vec<f32> = edge_img
                .pixels()
                .iter()
                .map(|y| y - brightness_offset)
                .collect();

            let luma_chars =
                pixels_to_chars(&luma_pixels, out_img_width, out_img_height, &font, convert);

            let edge_chars = pixels_to_chars(
                &edge_pixels,
                out_img_width,
                out_img_height,
                &font,
                direction_convert,
            );

            luma_chars
                .iter()
                .zip(edge_chars)
                .map(|(&luma, edge)| if edge == ' ' { luma } else { edge })
                .collect()
        }
    };

    (0..out_height * out_width)
        .step_by(out_width)
        .map(|i| chars[i..i + out_width].to_vec())
        .collect()
}

pub fn char_rows_to_string(char_rows: &[Vec<char>]) -> String {
    char_rows
        .iter()
        .map(|row| row.iter().collect())
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn char_rows_to_terminal_color_string(char_rows: &[Vec<char>], img: &DynamicImage) -> String {
    let (n_cols, n_rows) = (char_rows[0].len(), char_rows.len());
    let color_resized_image = img
        .resize_exact(n_cols as u32, n_rows as u32, FilterType::Nearest)
        .to_rgba32f();

    let colored_strings: Vec<String> = char_rows
        .into_iter()
        .flatten()
        .zip(color_resized_image.pixels())
        .map(|(c, Rgba([r, g, b, a]))| {
            let intensity = a * 255.;
            format!(
                "{}",
                c.to_string().truecolor(
                    (*r * intensity) as u8,
                    (*g * intensity) as u8,
                    (*b * intensity) as u8
                )
            )
        })
        .collect();

    (0..n_rows * n_cols)
        .step_by(n_cols)
        .map(|i| colored_strings[i..i + n_cols].join(""))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn char_rows_to_html_color_string(char_rows: &[Vec<char>], img: &DynamicImage) -> String {
    let (n_cols, n_rows) = (char_rows[0].len(), char_rows.len());
    let color_resized_image = img
        .resize_exact(n_cols as u32, n_rows as u32, FilterType::Nearest)
        .to_rgba8();

    let colored_strings: Vec<String> = char_rows
        .into_iter()
        .flatten()
        .zip(color_resized_image.pixels())
        .map(|(c, Rgba([r, g, b, a]))| {
            format!(
                "<span style=\"color: rgba({}, {}, {}, {})\">{}</span>",
                r, g, b, a, c
            )
        })
        .collect();

    (0..n_rows * n_cols)
        .step_by(n_cols)
        .map(|i| colored_strings[i..i + n_cols].join(""))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn char_rows_to_bitmap(char_rows: &[Vec<char>], font: &Font) -> DynamicImage {
    let out_width = (char_rows[0].len() * font.width) as u32;
    let out_height = (char_rows.len() * font.height) as u32;
    let mut image = GrayImage::new(out_width, out_height);

    for (j, row) in char_rows.iter().enumerate() {
        for (i, chr) in row.iter().enumerate() {
            let x_offset = i * font.width;
            let y_offset = j * font.height;
            let bitmap = &font.char_map.get(&chr).unwrap().bitmap;
            for y in 0..font.height {
                for x in 0..font.width {
                    let pixel = Luma([(255. * bitmap[y * font.width + x]) as u8]);
                    image.put_pixel((x + x_offset) as u32, (y + y_offset) as u32, pixel);
                }
            }
        }
    }

    DynamicImage::ImageLuma8(image)
}

pub fn char_rows_to_color_bitmap(
    char_rows: &[Vec<char>],
    font: &Font,
    img: &DynamicImage,
) -> DynamicImage {
    let (n_cols, n_rows) = (char_rows[0].len(), char_rows.len());
    let color_resized_image = img
        .resize_exact(n_cols as u32, n_rows as u32, FilterType::Nearest)
        .to_rgba32f();
    let pixels: Vec<&Rgba<f32>> = color_resized_image.pixels().collect();

    let out_width = (n_cols * font.width) as u32;
    let out_height = (n_rows * font.height) as u32;
    let mut image = RgbImage::new(out_width, out_height);

    for (j, row) in char_rows.iter().enumerate() {
        for (i, chr) in row.iter().enumerate() {
            let x_offset = i * font.width;
            let y_offset = j * font.height;
            let Rgba([r, g, b, a]) = pixels[j * n_cols as usize + i];
            let bitmap = &font.char_map.get(&chr).unwrap().bitmap;
            for y in 0..font.height {
                for x in 0..font.width {
                    let intensity = bitmap[y * font.width + x] * a * 255.;
                    let pixel = Rgb([
                        (*r * intensity) as u8,
                        (*g * intensity) as u8,
                        (*b * intensity) as u8,
                    ]);
                    image.put_pixel((x + x_offset) as u32, (y + y_offset) as u32, pixel);
                }
            }
        }
    }

    DynamicImage::ImageRgb8(image)
}
