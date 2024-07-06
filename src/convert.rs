use colored::Colorize;
use rand::prelude::ThreadRng;
use rand::{thread_rng, Rng};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};
use std::thread;

use image::imageops::FilterType::{self, Triangle};
use image::{DynamicImage, GenericImageView, GrayImage, Luma, Rgb, RgbImage};

use crate::font::Font;
use crate::metrics::{
    avg_color_score, dot_score, jaccard_score, movement_toward_clear, occlusion_score, Metric,
};

pub type Converter = fn(&Font, &[f32], &mut ThreadRng, f32) -> char;
pub enum ConversionAlgorithm {
    Base,
    Edge,
    TwoPass,
}

pub fn score_convert(
    score_fn: Metric,
    font: &Font,
    chunk: &[f32],
    rng: &mut ThreadRng,
    noise_scale: f32,
) -> char {
    let scores: HashMap<char, f32> = font
        .chars
        .iter()
        .map(|c| {
            let score = score_fn(&chunk, &c.bitmap);
            let noise = rng.gen::<f32>() * noise_scale;
            (c.value, score + noise)
        })
        .collect();
    *scores
        .keys()
        .max_by(|a, b| scores[a].partial_cmp(&scores[b]).unwrap_or(Ordering::Equal))
        .unwrap()
}

pub fn dot_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(dot_score, font, chunk, rng, noise_scale)
}

pub fn jaccard_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(jaccard_score, font, chunk, rng, noise_scale)
}

pub fn occlusion_convert(
    font: &Font,
    chunk: &[f32],
    rng: &mut ThreadRng,
    noise_scale: f32,
) -> char {
    score_convert(occlusion_score, font, chunk, rng, noise_scale)
}

pub fn color_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(avg_color_score, font, chunk, rng, noise_scale)
}

pub fn clear_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(movement_toward_clear, font, chunk, rng, noise_scale)
}

pub fn fast_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    let intensity = chunk.iter().sum::<f32>();
    let noise = rng.gen::<f32>() * noise_scale;
    let index = (intensity + noise) as usize;
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

pub fn direction_and_intensity_convert(
    font: &Font,
    chunk: &[f32],
    rng: &mut ThreadRng,
    noise_scale: f32,
) -> char {
    let max_direction = (font.width * font.height * 4) as f32; // direction should never be bigger than this
    let (x_dir, y_dir) = chunk_direction(chunk, font.width, font.height);
    let intensity = chunk.iter().sum::<f32>();

    let scores: Vec<f32> = font
        .chars
        .iter()
        .map(|c| {
            let grad =
                -((x_dir - c.direction.0).powf(2.) + (y_dir - c.direction.1).powf(2.)).powf(0.5);
            let score = (max_direction - grad) / (1. + (intensity - c.intensity).powf(2.));
            let noise = rng.gen::<f32>() * noise_scale;
            score + noise
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

pub fn direction_convert(
    font: &Font,
    chunk: &[f32],
    rng: &mut ThreadRng,
    noise_scale: f32,
) -> char {
    let (x_dir, y_dir) = chunk_direction(chunk, font.width, font.height);

    let scores: Vec<f32> = font
        .chars
        .iter()
        .map(|c| {
            let score = -((x_dir - c.direction.0).powi(2) + (y_dir - c.direction.1).powi(2)).sqrt();
            let noise = rng.gen::<f32>() * noise_scale;
            score + noise
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

pub fn get_converter(metric: &str) -> Converter {
    match &metric[..] {
        "dot" => dot_convert,
        "jaccard" => jaccard_convert,
        "occlusion" => occlusion_convert,
        "color" => color_convert,
        "clear" => clear_convert,
        "fast" => fast_convert,
        "grad" => direction_and_intensity_convert,
        "direction" => direction_convert,
        "direction-and-intensity" => direction_and_intensity_convert,
        _ => panic!("Unsupported metric {}", metric),
    }
}

pub fn get_conversion_algorithm(algorithm: &str) -> ConversionAlgorithm {
    match &algorithm[..] {
        "base" => ConversionAlgorithm::Base,
        "edge" => ConversionAlgorithm::Edge,
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

pub fn chunks_to_chars(
    font: &Font,
    chunks: &Vec<Vec<f32>>,
    convert: Converter,
    noise_scale: f32,
    n_threads: usize,
) -> Vec<char> {
    let mut chars: Vec<char> = Vec::with_capacity(chunks.len());
    if n_threads > 1 {
        let (tx, rx) = mpsc::channel();
        let font_arc = Arc::new(font.clone());

        let chunk_len = chunks.len() / n_threads + 1;
        for i in 0..n_threads {
            let _tx = tx.clone();
            let _font = font_arc.clone();
            let _chunk = chunks[i * chunk_len..((i + 1) * chunk_len).min(chunks.len())].to_vec();
            thread::spawn(move || {
                let mut rng = thread_rng();
                let chars: Vec<char> = _chunk
                    .iter()
                    .map(|chunk| convert(&_font, chunk, &mut rng, noise_scale))
                    .collect();
                _tx.send((i, chars)).unwrap();
            });
        }
        drop(tx);

        let mut char_vecs: Vec<(usize, Vec<char>)> = rx.iter().collect();
        char_vecs.sort();

        for (_, char_vec) in char_vecs {
            chars.extend(char_vec);
        }
    } else {
        // TODO make pixel_chunk_to_ascii a parameter so that "fast" can be passed in
        let mut rng = thread_rng();
        chars = chunks
            .iter()
            .map(|chunk| convert(&font, chunk, &mut rng, noise_scale))
            .collect();
    }

    chars
}

pub fn pixels_to_chars(
    pixels: &[f32],
    width: usize,
    height: usize,
    font: &Font,
    convert: Converter,
    noise_scale: f32,
    n_threads: usize,
) -> Vec<char> {
    let chunks = pixels_to_chunks(pixels, width, height, font.width, font.height);
    chunks_to_chars(font, &chunks, convert, noise_scale, n_threads)
}

pub fn img_to_char_rows(
    font: &Font,
    img: &DynamicImage,
    convert: Converter,
    out_width: usize,
    brightness_offset: f32,
    noise_scale: f32,
    n_threads: usize,
    algorithm: &ConversionAlgorithm,
) -> Vec<Vec<char>> {
    let (width, height) = img.dimensions();

    let out_height = (height as f64
        * (out_width as f64 / width as f64)
        * (font.width as f64 / font.height as f64))
        .round() as usize;

    let (out_img_width, out_img_height) = (out_width * font.width, out_height * font.height);
    let resized_image = img.resize_exact(
        out_img_width as u32,
        out_img_height as u32,
        FilterType::Nearest,
    );

    let chars: Vec<char> = match algorithm {
        ConversionAlgorithm::Base => {
            let pixels: Vec<f32> = resized_image
                .to_luma8()
                .pixels()
                .map(|&Luma([x])| (x as f32 - brightness_offset) / 255.)
                .collect();

            pixels_to_chars(
                &pixels,
                out_img_width as usize,
                out_img_height as usize,
                &font,
                convert,
                noise_scale,
                n_threads,
            )
        }
        ConversionAlgorithm::Edge => {
            let edge_detected = img
                .filter3x3(&[0., -1., 0., -1., 4., -1., 0., -1., 0.])
                .resize_exact(out_img_width as u32, out_img_height as u32, Triangle); // this resize is critical!
            let pixels: Vec<f32> = resized_image
                .to_luma8()
                .pixels()
                .zip(edge_detected.to_luma8().pixels())
                .map(|(&Luma([a]), &Luma([b]))| {
                    (a as f32 / 4. + b as f32 - brightness_offset) as f32 / 255.
                })
                .collect();

            pixels_to_chars(
                &pixels,
                out_img_width as usize,
                out_img_height as usize,
                &font,
                convert,
                noise_scale,
                n_threads,
            )
        }
        ConversionAlgorithm::TwoPass => {
            let luma_pixels: Vec<f32> = resized_image
                .to_luma8()
                .pixels()
                .map(|&Luma([x])| (x as f32 - brightness_offset) / 255.)
                .collect();

            let edge_detected = img
                .blur(1.0)
                .filter3x3(&[0., -1., 0., -1., 4., -1., 0., -1., 0.])
                .resize_exact(out_img_width as u32, out_img_height as u32, Triangle); // this resize is critical!
            let edge_detection_pixels: Vec<f32> = edge_detected
                .to_luma8()
                .pixels()
                .map(|&Luma([a])| (a as f32 - brightness_offset) / 255.)
                .collect();

            let luma_chars = pixels_to_chars(
                &luma_pixels,
                out_img_width as usize,
                out_img_height as usize,
                &font,
                convert,
                noise_scale,
                n_threads,
            );

            let edge_detection_chars = pixels_to_chars(
                &edge_detection_pixels,
                out_img_width as usize,
                out_img_height as usize,
                &font,
                direction_convert,
                noise_scale,
                n_threads,
            );

            luma_chars
                .iter()
                .zip(edge_detection_chars)
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
        .to_rgb8();

    let colored_strings: Vec<String> = char_rows
        .into_iter()
        .flatten()
        .zip(color_resized_image.pixels())
        .map(|(c, Rgb([r, g, b]))| format!("{}", c.to_string().truecolor(*r, *g, *b)))
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
        .to_rgb8();

    let colored_strings: Vec<String> = char_rows
        .into_iter()
        .flatten()
        .zip(color_resized_image.pixels())
        .map(|(c, Rgb([r, g, b]))| {
            format!(
                "<span style=\"color: rgb({}, {}, {})\">{}</span>",
                r, g, b, c
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
        .to_rgb8();
    let pixels: Vec<&Rgb<u8>> = color_resized_image.pixels().collect();

    let out_width = (n_cols * font.width) as u32;
    let out_height = (n_rows * font.height) as u32;
    let mut image = RgbImage::new(out_width, out_height);

    for (j, row) in char_rows.iter().enumerate() {
        for (i, chr) in row.iter().enumerate() {
            let x_offset = i * font.width;
            let y_offset = j * font.height;
            let Rgb([r, g, b]) = pixels[j * n_cols as usize + i];
            let bitmap = &font.char_map.get(&chr).unwrap().bitmap;
            for y in 0..font.height {
                for x in 0..font.width {
                    let intensity = bitmap[y * font.width + x];
                    let pixel = Rgb([
                        (*r as f32 * intensity) as u8,
                        (*g as f32 * intensity) as u8,
                        (*b as f32 * intensity) as u8,
                    ]);
                    image.put_pixel((x + x_offset) as u32, (y + y_offset) as u32, pixel);
                }
            }
        }
    }

    DynamicImage::ImageRgb8(image)
}
