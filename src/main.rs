use crate::font::Font;
use crate::metrics::{
    avg_color_score, dot_score, jaccard_score, movement_toward_clear, occlusion_score, Metric,
};
use clap::Parser;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Instant, Duration};
use indicatif::ProgressIterator;

use log::info;

mod convert;
mod font;
mod gif;
mod metrics;

#[derive(Parser)]
struct Cli {
    image_path: String,
    #[clap(short, long, default_value_t = String::from("fonts/kourier.bdf"))]
    font_path: String,
    #[clap(short, long, default_value_t = String::from("alphabets/alphabet.txt"))]
    alphabet_path: String,
    #[clap(short, long, default_value_t = 150)]
    width: usize,
    #[clap(short, long, default_value_t = String::from("dot"))]
    metric: String,
    #[clap(short, long, default_value_t = 1)]
    threads: usize,
    #[clap(short, long, default_value_t = 128.0)]
    brightness_offset: f32,
    #[clap(short, long, default_value_t = 0.0)]
    noise_scale: f32,
    #[clap(short, long)]
    out_path: Option<String>,
    #[clap(long, default_value_t = 60.0)]
    fps: f64
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    let width = args.width;
    info!("width          {}", width);

    let image_path = Path::new(&args.image_path);
    info!("image path     {:?}", image_path);
    let extension = image_path.extension().unwrap();

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

    info!("rendering...");
    let mut output: Vec<String> = Vec::new();
    if metric == "fast" {
        if extension == "gif" {
            let gif = gif::read_gif(image_path);
            for img in gif.iter().progress() {
                let ascii = convert::img_to_ascii_fast(&font, &img, width);
                output.push(ascii);
            }
        } else {
            let img = image::open(image_path).unwrap();
            let ascii = convert::img_to_ascii_fast(&font, &img, width);
            output.push(ascii);
        }
    } else {
        let threads = args.threads;
        info!("threads        {}", threads);

        let brightness_offset = args.brightness_offset;
        info!("brightness     {}", brightness_offset);

        let noise_scale = args.noise_scale;
        info!("noise scale    {}", noise_scale);

        let metric_fn: Option<Metric> = match &metric[..] {
            "jaccard" => Some(jaccard_score),
            "dot" => Some(dot_score),
            "occlusion" => Some(occlusion_score),
            "color" => Some(avg_color_score),
            "clear" => Some(movement_toward_clear),
            _ => None,
        };
        // if the user specified a metric, don't fall back to the default
        let metric = metric_fn.expect(&format!("Unsupported metric {}", metric));

        if extension == "gif" {
            let gif = gif::read_gif(image_path);
            for img in gif.iter().progress() {
                let ascii = convert::img_to_ascii(&font, &img, metric, width, brightness_offset, noise_scale, threads);
                output.push(ascii);
            }
        } else {
            let img = image::open(image_path).unwrap();
            let ascii = convert::img_to_ascii(&font, &img, metric, width, brightness_offset, noise_scale, threads);
            output.push(ascii);
        }
    }
    info!("done!");

    if let Some(path) = out_path {
        let json = serde_json::to_string(&output).unwrap();
        fs::write(path, json).unwrap();
    } else {
        if extension == "gif" {
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
