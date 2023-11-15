use crate::convert::{get_converter};
use crate::font::Font;
use crate::gif::write_gif;
use crate::progress::default_progress_bar;
use crate::convert::{char_rows_to_bitmap, char_rows_to_color_bitmap, char_rows_to_string, char_rows_to_terminal_color_string, char_rows_to_html_color_string};

use clap::Parser;
use image::{DynamicImage, GenericImageView};
use indicatif::ProgressIterator;
use std::collections::HashMap;
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
    #[clap(short, long, default_value_t = String::from("courier"))]
    font: String,
    #[clap(short, long, default_value_t = String::from("alphabet"))]
    alphabet: String,
    #[clap(short, long, default_value_t = 150)]
    width: usize,
    #[clap(short, long, default_value_t = String::from("grad"))]
    metric: String,
    #[clap(short, long, default_value_t = 1)]
    threads: usize,
    #[clap(long)]
    no_color: bool,
    #[clap(short, long, default_value_t = 0.0)]
    brightness_offset: f32,
    #[clap(short, long, default_value_t = 0.0)]
    noise_scale: f32,
    #[clap(short, long)]
    out_path: Option<String>,
    #[clap(long, default_value_t = 30.0)]
    fps: f64,
    #[clap(long)]
    no_edge_detection: bool
}

const ALPHABETS: [(&str, &str); 6] = [
    ("alphabet", include_str!("../alphabets/alphabet.txt")),
    ("letters", include_str!("../alphabets/letters.txt")),
    ("lowercase", include_str!("../alphabets/lowercase.txt")),
    ("minimal", include_str!("../alphabets/minimal.txt")),
    ("symbols", include_str!("../alphabets/symbols.txt")),
    ("uppercase", include_str!("../alphabets/uppercase.txt")),
];

const FONTS: [(&str, &str); 2] = [
    ("courier", include_str!("../fonts/courier.bdf")),
    ("bitocra-13", include_str!("../fonts/bitocra-13.bdf")),
];

fn main() {
    env_logger::init();

    let args = Cli::parse();

    let width = args.width;
    info!("width          {}", width);

    let image_path = Path::new(&args.image_path);
    info!("image path     {:?}", image_path);
    let in_extension = image_path.extension().unwrap();

    let alphabet_str = &args.alphabet;
    let alphabet_map: HashMap<&str, &str> = ALPHABETS.iter().cloned().collect();
    let alphabet: Vec<char> = if alphabet_map.contains_key(&alphabet_str.as_ref()) {
        info!("alphabet name  {:?}", alphabet_str);
        alphabet_map
            .get(&alphabet_str.as_ref())
            .unwrap()
            .chars()
            .collect()
    } else {
        let alphabet_path = Path::new(alphabet_str);
        info!("alphabet path  {:?}", alphabet_path);
        fs::read(&alphabet_path)
            .unwrap()
            .iter()
            .map(|&b| b as char)
            .collect()
    };
    info!("alphabet       [{}]", alphabet.iter().collect::<String>());

    let font_str = &args.font;
    let font_map: HashMap<&str, &str> = FONTS.iter().cloned().collect();
    let font: font::Font = if font_map.contains_key(&font_str.as_ref()) {
        info!("font name      {:?}", font_str);
        let font_data = font_map.get(&font_str.as_ref()).unwrap();
        Font::from_bdf_stream(font_data.as_bytes(), &alphabet)
    } else {
        let font_path = Path::new(font_str);
        info!("font path      {:?}", font_path);
        Font::from_bdf(font_path, &alphabet)
    };

    let metric = args.metric;
    info!("metric         {}", metric);

    let out_path = args.out_path.as_ref().map(|name| Path::new(name));
    info!("out path       {:?}", out_path);

    let fps = args.fps;
    info!("fps            {}", fps);

    let color = !args.no_color;
    info!("color          {}", color);

    let brightness_offset = args.brightness_offset;
    info!("brightness     {}", brightness_offset);

    let noise_scale = args.noise_scale;
    info!("noise scale    {}", noise_scale);

    let threads = args.threads;
    info!("threads        {}", threads);

    let edge_detection = !args.no_edge_detection;
    info!("edge detection {}", edge_detection);

    let convert = get_converter(&metric);
    // info!("converter      {:?}", convert);

    info!("converting frames to ascii...");
    let frames: Vec<DynamicImage> = if in_extension == "gif" {
        let gif = gif::read_gif(image_path);
        gif.iter().cloned().collect()
    } else {
        let img = image::open(image_path).unwrap();
        vec![img]
    };

    let mut frame_char_rows: Vec<Vec<Vec<char>>> = Vec::new();
    let progress = default_progress_bar("Frames", frames.len());
    for img in frames.iter().progress_with(progress) {
        let ascii = convert::img_to_char_rows(
            &font,
            &img,
            convert,
            width,
            brightness_offset,
            noise_scale,
            threads,
            edge_detection
        );
        frame_char_rows.push(ascii);
    }

    if let Some(path) = out_path {
        let out_extension = path.extension().unwrap();

        if out_extension == "json" {
            let out_frames: Vec<String> = if color {
                frame_char_rows.iter().zip(frames).map(|(char_rows, frame)| char_rows_to_html_color_string(char_rows, &frame)).collect()
            } else {
                frame_char_rows.iter().map(|char_rows| char_rows_to_string(char_rows)).collect()
            };
            let json = serde_json::to_string(&out_frames).unwrap();
            fs::write(path, json).unwrap();
        } else if out_extension == "gif" {
            info!("converting ascii strings to bitmaps...");
            let progress = default_progress_bar("Frames", frame_char_rows.len());
            let out_frames: Vec<DynamicImage> = if color {
                frame_char_rows
                .iter()
                .zip(frames)
                .progress_with(progress)
                .map(|(char_rows, frame)| char_rows_to_color_bitmap(&char_rows, &font, &frame))
                .collect()
            } else {
                frame_char_rows
                .iter()
                .progress_with(progress)
                .map(|char_rows| char_rows_to_bitmap(&char_rows, &font))
                .collect()
            };
            write_gif(path, &out_frames, fps);
        } else {
            let img = if color {
                char_rows_to_color_bitmap(&frame_char_rows[0], &font, &frames[0])
            } else {
                char_rows_to_bitmap(&frame_char_rows[0], &font)
            };
            img.save(path).unwrap();
        }
    } else {
        let out_frames: Vec<String> = if color {
            frame_char_rows.iter().zip(frames).map(|(char_rows, frame)| char_rows_to_terminal_color_string(char_rows, &frame)).collect()
        } else {
            frame_char_rows.iter().map(|char_rows| char_rows_to_string(char_rows)).collect()
        };

        if in_extension == "gif" {
            loop {
                for frame in &out_frames {
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
            println!("{}", out_frames[0]);
        }
    }
}
