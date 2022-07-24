use colored::Colorize;
use rand::prelude::ThreadRng;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{mpsc, Arc};
use std::thread;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Luma, GrayImage, Rgb};

use crate::font::Font;
use crate::metrics::{
    avg_color_score, dot_score, jaccard_score, movement_toward_clear, occlusion_score, Metric,
};

pub type Converter = fn(&Font, &[f32], &mut ThreadRng, f32) -> char;

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
        .max_by(|a, b| scores[a].partial_cmp(&scores[b]).unwrap())
        .unwrap()
}

pub fn dot_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(dot_score, font, chunk, rng, noise_scale)
}

pub fn jaccard_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    score_convert(jaccard_score, font, chunk, rng, noise_scale)
}

pub fn occlusion_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
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

pub fn grad_convert(font: &Font, chunk: &[f32], rng: &mut ThreadRng, noise_scale: f32) -> char {
    let max_gradient = (font.width * font.height * 4) as f32; // gradient should never be bigger than this

    let intensity = chunk.iter().sum::<f32>();
    let mut x_grad = 0.0;
    let mut y_grad = 0.0;
    for i in 0..font.height {
        for j in 0..font.width - 1 {
            if chunk[i * font.width + 1 + j] > chunk[i * font.width + j] {
                x_grad += 1.;
            }
        }
    }
    for i in 0..font.height - 1 {
        for j in 0..font.width {
            if chunk[(i + 1) * font.width + j] > chunk[i * font.width + j] {
                y_grad += 1.;
            }
        }
    }

    let scores: Vec<f32> = font
        .intensities
        .iter()
        .zip(font.grads.iter())
        .map(|(char_intensity, (char_x_grad, char_y_grad))| {
            let grad = ((x_grad - char_x_grad).powf(2.) + (y_grad - char_y_grad).powf(2.)).powf(0.5);
            let score = (max_gradient - grad) / (1. + (intensity - char_intensity).powf(2.));
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
    let convert: Option<Converter> = match &metric[..] {
        "dot" => Some(dot_convert),
        "jaccard" => Some(jaccard_convert),
        "occlusion" => Some(occlusion_convert),
        "color" => Some(color_convert),
        "clear" => Some(clear_convert),
        "fast" => Some(fast_convert),
        "grad" => Some(grad_convert),
        _ => None,
    };
    convert.expect(&format!("Unsupported metric {}", metric))
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
                let chars: Vec<char> = _chunk.iter().map(|chunk| convert(&_font, chunk, &mut rng, noise_scale)).collect();
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
        chars = chunks.iter().map(|chunk| convert(&font, chunk, &mut rng, noise_scale)).collect();
    }

    chars
}

pub fn img_to_ascii(
    font: &Font,
    img: &DynamicImage,
    convert: Converter,
    out_width: usize,
    color: bool,
    brightness_offset: f32,
    noise_scale: f32,
    n_threads: usize
) -> String {
    let (width, height) = img.dimensions();

    let resize_width = out_width * font.width;
    let out_height = (((resize_width * height as usize) as f64)
        / ((width as usize * font.height) as f64))
        .round() as usize;
    let resize_height = out_height * font.height;
    let img = img.resize_exact(
        resize_width as u32,
        resize_height as u32,
        FilterType::Nearest,
    );

    let img_buffer = img.to_luma8();

    let pixels: Vec<f32> = img_buffer
        .pixels()
        .map(|&Luma([x])| (x as f32 - brightness_offset) / 255.)
        .collect();

    let chunks = pixels_to_chunks(&pixels, resize_width, resize_height, font.width, font.height);
    let chars = chunks_to_chars(font, &chunks, convert, noise_scale, n_threads);

    let strings: Vec<String> = if color {
        let color_resized_image = img.resize_exact(
            (resize_width / font.width) as u32,
            (resize_height / font.height) as u32,
            FilterType::Nearest
        ).to_rgb8();
    
        chars.iter().zip(color_resized_image.pixels()).map(
            |(c, Rgb([r, g, b]))| format!("{}", format!("{}", c).truecolor(*r, *g, *b))
        ).collect()
    } else {
        chars.iter().map(|c| c.to_string()).collect()
    };

    let mut string_rows: Vec<String> = Vec::with_capacity(out_height);
    for j in 0..out_height {
        let start = j * out_width;
        let end = start + out_width;
        let row = strings[start..end].join("");
        string_rows.push(row);
    }
    let result = string_rows.join("\n");

    result
}

pub fn ascii_to_bitmap(
    font: &Font,
    ascii: &str,
) -> DynamicImage {
    let lines: Vec<&str> = ascii.lines().collect();
    let width = (lines[0].len() * font.width) as u32;
    let height = (lines.len() * font.height) as u32;
    let mut image = GrayImage::new(width, height);

    for (j, line) in lines.iter().enumerate() {
        for (i, chr) in line.chars().enumerate() {
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