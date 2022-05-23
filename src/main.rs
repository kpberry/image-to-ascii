use crate::convert::{ascii_to_bitmap, get_converter};
use crate::font::Font;
use crate::gif::write_gif;
use crate::progress::default_progress_bar;

use clap::Parser;
use image::DynamicImage;
use indicatif::ProgressIterator;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::info;

mod convert;
mod font;
mod gif;
mod metrics;
mod progress;

#[derive(Parser)]
struct Cli {
    image_path: String,
    #[clap(short, long, default_value_t = String::from("fonts/kourier.bdf"))]
    font_path: String,
    #[clap(short, long, default_value_t = String::from("alphabets/alphabet.txt"))]
    alphabet_path: String,
    #[clap(short, long, default_value_t = 150)]
    width: usize,
    #[clap(short, long, default_value_t = String::from("grad"))]
    metric: String,
    #[clap(short, long, default_value_t = 1)]
    threads: usize,
    #[clap(short, long, default_value_t = 128.0)]
    brightness_offset: f32,
    #[clap(short, long, default_value_t = 0.0)]
    noise_scale: f32,
    #[clap(short, long)]
    out_path: Option<String>,
    #[clap(long, default_value_t = 30.0)]
    fps: f64,
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    let width = args.width;
    info!("width          {}", width);

    let image_path = Path::new(&args.image_path);
    info!("image path     {:?}", image_path);
    let in_extension = image_path.extension().unwrap();

    let alphabet_path = Path::new(&args.alphabet_path);
    info!("alphabet path  {:?}", alphabet_path);
    let alphabet: Vec<char> = fs::read(&alphabet_path)
        .unwrap()
        .iter()
        .map(|&b| b as char)
        .collect();
    info!("alphabet       [{}]", alphabet.iter().collect::<String>());

    let font_path = Path::new(&args.font_path);
    info!("font path      {:?}", font_path);
    let font = Font::from_bdf(Path::new("fonts/kourier.bdf"), &alphabet);

    let metric = args.metric;
    info!("metric         {}", metric);

    let out_path = args.out_path.as_ref().map(|name| Path::new(name));
    info!("out path       {:?}", out_path);

    let fps = args.fps;
    info!("fps            {}", fps);

    let brightness_offset = args.brightness_offset;
    info!("brightness     {}", brightness_offset);

    let noise_scale = args.noise_scale;
    info!("noise scale    {}", noise_scale);

    let threads = args.threads;
    info!("threads        {}", threads);

    let convert = get_converter(&metric);
    // info!("converter      {:?}", convert);

    info!("converting frames to ascii...");
    let mut output: Vec<String> = Vec::new();

    let frames: Vec<DynamicImage> = if in_extension == "gif" {
        let gif = gif::read_gif(image_path);
        gif.iter().cloned().collect()
    } else {
        let img = image::open(image_path).unwrap();
        vec![img]
    };

    let progress = default_progress_bar("Frames", frames.len());
    for img in frames.iter().progress_with(progress) {
        let ascii = convert::img_to_ascii(
            &font,
            &img,
            convert,
            width,
            brightness_offset,
            noise_scale,
            threads,
        );
        output.push(ascii);
    }

    if let Some(path) = out_path {
        let out_extension = path.extension().unwrap();

        if out_extension == "json" {
            let json = serde_json::to_string(&output).unwrap();
            fs::write(path, json).unwrap();
        } else if out_extension == "gif" {
            info!("converting ascii strings to bitmaps...");
            let progress = default_progress_bar("Frames", output.len());
            let frames: Vec<DynamicImage> = output
                .iter()
                .progress_with(progress)
                .map(|frame| ascii_to_bitmap(&font, &frame))
                .collect();
            write_gif(path, &frames, fps);
        }
    } else {
        if in_extension == "gif" {
            loop {
                for frame in &output {
                    let t0 = Instant::now();
                    println!("{}[2J{}", 27 as char, frame);
                    let elapsed = t0.elapsed().as_secs_f64();
                    let delay = (1.0 / fps) - elapsed;
                    if delay > 0.0 {
                        sleep(Duration::from_secs_f64(delay));
                    }
                }
            }
        } else {
            println!("{}", output[0]);
        }
    }
}
