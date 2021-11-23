use std::fs::File;
use std::path::Path;
use image::{AnimationDecoder, DynamicImage};
use image::codecs::gif::{GifDecoder};

fn read_gif(path: &Path) -> Vec<DynamicImage> {
    let fp = File::open(path).unwrap();
    let decoder = GifDecoder::new(fp).unwrap();
    let frames = decoder.into_frames();
    let frames = frames.collect_frames().expect("error decoding gif");
    frames.iter().map(|frame| DynamicImage::ImageRgba8(frame.buffer().clone())).collect()
}
