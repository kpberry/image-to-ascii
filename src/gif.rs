use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage};
use std::fs::File;
use std::io::Read;
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
