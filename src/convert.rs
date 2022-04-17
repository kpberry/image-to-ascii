use std::collections::HashMap;
use std::sync::{mpsc, Arc};
use std::thread;
use rand::{Rng, thread_rng};

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Luma};

use crate::{Font, Metric};

pub fn pixel_chunk_to_ascii(font: &Font, chunk: &[f32], score_fn: Metric, noise_scale: f32) -> char {
    let mut rng = thread_rng();
    let scores: HashMap<char, f32> = font
        .chars
        .iter()
        .map(|c| (c.value, score_fn(&chunk, &c.bitmap) + rng.gen::<f32>() * noise_scale))
        .collect();
    *scores
        .keys()
        .max_by(|a, b| scores[a].partial_cmp(&scores[b]).unwrap())
        .unwrap()
}

pub fn pixels_to_ascii(
    font: &Font,
    pixels: Vec<f32>,
    metric: Metric,
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
    noise_scale: f32,
    n_threads: usize,
) -> String {
    let chunk_size = font.width * font.height;
    let vertical_chunks = height / font.height;
    let horizontal_chunks = width / font.width;

    // not the simplest way of doing this, but should give good cache performance and avoids
    // multiplications/divisions
    let mut chunks: Vec<Vec<f32>> = Vec::with_capacity(vertical_chunks * horizontal_chunks);
    let mut y_offset = 0;
    let mut x_offset = 0;
    for _ in 0..vertical_chunks {
        let mut chunk_row: Vec<Vec<f32>> = (0..horizontal_chunks)
            .map(|_| Vec::with_capacity(chunk_size))
            .collect();

        for _ in 0..font.height {
            for x in 0..horizontal_chunks {
                let start = y_offset + x_offset;
                let end = start + font.width;
                let chunk_pixel_row = &pixels[start..end];
                chunk_row[x].extend(chunk_pixel_row);
                x_offset += font.width;
            }
            y_offset += width;
            x_offset = 0;
        }

        chunks.extend(chunk_row);
    }

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
                let chars: Vec<char> = _chunk
                    .iter()
                    .map(|chunk| pixel_chunk_to_ascii(&_font, chunk, metric, noise_scale))
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
        chars = chunks
            .iter()
            .map(|chunk| pixel_chunk_to_ascii(&font, chunk, metric, noise_scale))
            .collect();
    }

    let mut char_rows: Vec<Vec<char>> = Vec::new();
    for j in 0..out_height {
        let start = j * out_width;
        let end = start + out_width;
        let row = chars[start..end].iter().cloned().collect();
        char_rows.push(row);
    }
    let strings: Vec<String> = char_rows
        .iter()
        .map(|chars| chars.iter().collect())
        .collect();
    let result = strings.join("\n");

    result
}

pub fn img_to_ascii(
    font: &Font,
    img: &DynamicImage,
    metric: Metric,
    out_width: usize,
    brightness_offset: f32,
    noise_scale: f32,
    n_threads: usize,
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

    // sometimes makes the image look better
    // img.invert();

    // edge detection
    // img = img.filter3x3(&vec![-1., 0., 1., -1., 0., 1., -1., 0., 1.]);

    let img = img.to_luma8();

    let pixels: Vec<f32> = img.pixels().map(|&Luma([x])| (x as f32 - brightness_offset) / 255.).collect();

    pixels_to_ascii(
        font,
        pixels,
        metric,
        resize_width,
        resize_height,
        out_width,
        out_height,
        noise_scale,
        n_threads,
    )
}

pub fn pixels_to_ascii_fast(
    font: &Font,
    pixels: Vec<f32>,
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> String {
    let chunk_size = font.width * font.height;
    let vertical_chunks = height / font.height;
    let horizontal_chunks = width / font.width;

    // not the simplest way of doing this, but should give good cache performance and avoids
    // multiplications/divisions
    let mut chunks: Vec<Vec<f32>> = Vec::with_capacity(vertical_chunks * horizontal_chunks);
    let mut y_offset = 0;
    let mut x_offset = 0;
    for _ in 0..vertical_chunks {
        let mut chunk_row: Vec<Vec<f32>> = (0..horizontal_chunks)
            .map(|_| Vec::with_capacity(chunk_size))
            .collect();

        for _ in 0..font.height {
            for x in 0..horizontal_chunks {
                let start = y_offset + x_offset;
                let end = start + font.width;
                let chunk_pixel_row = &pixels[start..end];
                chunk_row[x].extend(chunk_pixel_row);
                x_offset += font.width;
            }
            y_offset += width;
            x_offset = 0;
        }

        chunks.extend(chunk_row);
    }

    let chars: Vec<char> = chunks
        .iter()
        .map(|chunk| {
            let intensity = chunk.iter().sum::<f32>() as usize;
            font.intensity_chars[intensity].value
        })
        .collect();

    let mut char_rows: Vec<Vec<char>> = Vec::new();
    for j in 0..out_height {
        let start = j * out_width;
        let end = start + out_width;
        let row = chars[start..end].iter().cloned().collect();
        char_rows.push(row);
    }
    let strings: Vec<String> = char_rows
        .iter()
        .map(|chars| chars.iter().collect())
        .collect();
    let result = strings.join("\n");

    result
}

pub fn img_to_ascii_fast(font: &Font, img: &DynamicImage, out_width: usize) -> String {
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
    let img = img.to_luma8();

    let pixels: Vec<f32> = img.pixels().map(|&Luma([x])| x as f32 / 255.).collect();
    pixels_to_ascii_fast(
        font,
        pixels,
        resize_width,
        resize_height,
        out_width,
        out_height,
    )
}
