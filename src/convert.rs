use std::collections::HashMap;
use image::{DynamicImage, GenericImageView, Luma};
use image::imageops::FilterType;
use crate::{Font, metrics};

pub fn pixel_chunk_to_ascii(font: &Font, chunk: &[f32], score_fn: fn(&[f32], &[f32]) -> f32) -> char {
    let scores: HashMap<char, f32> = font.chars.iter().map(|c| (c.value, score_fn(&chunk, &c.bitmap))).collect();
    *scores.keys().max_by(|a, b| scores[a].partial_cmp(&scores[b]).unwrap()).unwrap()
}

pub fn pixels_to_ascii(font: &Font, pixels: Vec<f32>,
                       width: usize, height: usize,
                       out_width: usize, out_height: usize) -> String {
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
            .map(|_| Vec::with_capacity(chunk_size)).collect();

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

    let chars: Vec<char> = chunks.iter()
        .map(|chunk| pixel_chunk_to_ascii(font, chunk, metrics::movement_toward_clear)).collect();

    let mut char_rows: Vec<Vec<char>> = Vec::new();
    for j in 0..out_height {
        let start = j * out_width;
        let end = start + out_width;
        let row = chars[start..end].iter().cloned().collect();
        char_rows.push(row);
    }
    let strings: Vec<String> = char_rows.iter().map(|chars| chars.iter().collect()).collect();
    let result = strings.join("\n");

    result
}

pub fn img_to_ascii(font: &Font, img: &DynamicImage, out_width: usize) -> String {
    let (width, height) = img.dimensions();

    let resize_width = out_width * font.width;
    let out_height = (((resize_width * height as usize) as f64) / ((width as usize * font.height) as f64)).round() as usize;
    let resize_height = out_height * font.height;
    let mut img = img.resize_exact(resize_width as u32, resize_height as u32, FilterType::Triangle);

    // img.invert();
    // TODO for some reason this gives weird output? should edge detect
    // img = img.filter3x3(&vec![-1., 0., 1., -1., 0., 1., -1., 0., 1.]);

    let (width, height) = img.dimensions();
    let width = width as usize;
    let height = height as usize;
    let img = img.to_luma8();

    let mut pixels: Vec<f32> = img.pixels().map(|&Luma([x])| (x as f32) / 255.0).collect();
    let pixels_mean = (pixels.iter().fold(0.0, |acc, x| acc + x) as f64 / pixels.len() as f64) as f32;
    pixels = pixels.iter().map(|x| (x - pixels_mean) / pixels_mean).collect();

    pixels_to_ascii(font, pixels, width, height, out_width, out_height)
}