use crate::font::Font;
use crate::metrics::{
    avg_color_score, dot_score, jaccard_score, movement_toward_clear, occlusion_score, Metric,
};
use clap::Parser;
use std::fs;
use std::path::Path;

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
    threads: usize
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

    let threads = args.threads;
    info!("threads        {}", threads);

    if metric == "fast" {
        if extension == "gif" {
            let gif = gif::read_gif(image_path);
            for img in gif {
                let ascii = convert::img_to_ascii_fast(&font, &img, width);
                println!("{}[2J{}", 27 as char, ascii);
            }
        } else {
            let img = image::open(image_path).unwrap();
            let ascii = convert::img_to_ascii_fast(&font, &img, width);
            println!("{}", ascii);
        }
    } else {
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
            for img in gif {
                let ascii = convert::img_to_ascii(&font, &img, metric, width, threads);
                println!("{}[2J{}", 27 as char, ascii);
            }
        } else {
            let img = image::open(image_path).unwrap();
            let ascii = convert::img_to_ascii(&font, &img, metric, width, threads);
            println!("{}", ascii);
        }
    }
}
