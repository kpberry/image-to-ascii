use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, DynamicImage, Frame, Delay};
use indicatif::{ProgressBar, ProgressStyle, ProgressIterator};
use log::info;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

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
    info!("converting bitmaps to gif frames...");
    let mut encoder = GifEncoder::new(stream);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let delay = Delay::from_numer_denom_ms(1000, fps as u32);

    let progress_template = "[{wide_bar}] Frames: {pos}/{len} Time: ({elapsed}/{duration})";
    let progress = ProgressBar::new(frames.len() as u64);
    progress.set_style(ProgressStyle::default_bar().template(progress_template));
    let frames: Vec<Frame> = frames.iter().progress_with(progress).map(|f| Frame::from_parts(f.to_rgba8(), 0, 0, delay)).collect();
    
    info!("encoding gif frames...");
    let progress_template = "[{wide_bar}] Frames: {pos}/{len} Time: ({elapsed}/{duration})";
    let progress = ProgressBar::new(frames.len() as u64);
    progress.set_style(ProgressStyle::default_bar().template(progress_template));
    encoder.encode_frames(frames.into_iter().progress_with(progress)).unwrap();
}

pub fn write_gif(path: &Path, frames: &[DynamicImage], fps: f64) {
    let fp = File::create(path).unwrap();
    write_gif_to_stream(fp, frames, fps)
}