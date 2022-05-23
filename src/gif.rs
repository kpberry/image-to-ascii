use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Delay, DynamicImage, Frame};
use indicatif::ProgressIterator;
use log::info;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::progress::default_progress_bar;

pub fn read_gif_from_stream<R: Read>(stream: R) -> Vec<DynamicImage> {
    let decoder = GifDecoder::new(stream).unwrap();
    let frames = decoder.into_frames();
    let frames = frames.collect_frames().expect("error decoding gif");
    frames
        .iter()
        .map(|frame| DynamicImage::ImageRgba8(frame.buffer().clone()))
        .collect()
}

pub fn read_gif(path: &Path) -> Vec<DynamicImage> {
    let fp = File::open(path).unwrap();
    read_gif_from_stream(fp)
}

pub fn write_gif_to_stream<W: Write>(stream: W, frames: &[DynamicImage], fps: f64) {
    let mut encoder = GifEncoder::new(stream);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let delay = Delay::from_numer_denom_ms(1000, fps as u32);

    info!("converting bitmaps to gif frames...");
    let progress = default_progress_bar("Frames", frames.len());
    let frames: Vec<Frame> = frames
        .iter()
        .progress_with(progress)
        .map(|f| Frame::from_parts(f.to_rgba8(), 0, 0, delay))
        .collect();

    info!("encoding gif frames...");
    let progress = default_progress_bar("Frames", frames.len());
    encoder
        .encode_frames(frames.into_iter().progress_with(progress))
        .unwrap();
}

pub fn write_gif(path: &Path, frames: &[DynamicImage], fps: f64) {
    let fp = File::create(path).unwrap();
    write_gif_to_stream(fp, frames, fps)
}
