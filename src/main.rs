use crate::convert::get_converter;
use crate::convert::{
    char_rows_to_bitmap, char_rows_to_color_bitmap, char_rows_to_html_color_string,
    char_rows_to_string, char_rows_to_terminal_color_string,
};
use crate::font::Font;
use crate::gif::write_gif;
use crate::progress::default_progress_bar;

use ::image::DynamicImage;
use clap::Parser;
use convert::get_conversion_algorithm;
use image::LumaImage;
use indicatif::ProgressIterator;
use std::collections::HashMap;
use std::env::temp_dir;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::{info, warn};

mod convert;
mod font;
mod gif;
mod image;
mod metrics;
mod metrics_simd;
mod progress;

#[derive(Parser)]
struct Cli {
    image_path: String,
    #[clap(short, long, default_value_t = String::from("bitocra-13"))]
    font: String,
    #[clap(short, long, default_value_t = String::from("alphabet"))]
    alphabet: String,
    #[clap(short, long)]
    width: Option<usize>,
    #[clap(short, long, default_value_t = String::from("direction-and-intensity"))]
    metric: String,
    #[clap(long)]
    no_color: bool,
    #[clap(long)]
    invert: bool,
    #[clap(short = 'b', long, default_value_t = 0.0, allow_hyphen_values = true)]
    brightness_offset: f32,
    #[clap(short = 's', long, default_value_t = 0.25, allow_hyphen_values = true)]
    brightness_scale: f32,
    #[clap(short = 'g', long)]
    naive_grayscale: bool,
    #[clap(long, default_value_t = 1.0)]
    edge_brightness_scale: f32,
    #[clap(short, long)]
    out_path: Option<String>,
    #[clap(long, default_value_t = 30.0)]
    fps: f64,
    #[clap(short, long, default_value_t = String::from("edge-augmented"))]
    conversion_algorithm: String,
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

    // On Windows, this is unset, but the windows terminal should support truecolor.
    if std::env::var("COLORTERM").is_err() {
        std::env::set_var("COLORTERM", "truecolor");
    }

    let args = Cli::parse();

    let width = args.width;
    info!("width\t{:?}", width);

    let image_path = Path::new(&args.image_path);
    info!("image path\t{:?}", image_path);
    let in_extension = image_path.extension().unwrap();

    let alphabet_str = &args.alphabet;
    let alphabet_map: HashMap<&str, &str> = ALPHABETS.iter().cloned().collect();
    let alphabet: Vec<char> = if alphabet_map.contains_key(&alphabet_str.as_ref()) {
        info!("alphabet name\t{:?}", alphabet_str);
        alphabet_map
            .get(&alphabet_str.as_ref())
            .unwrap()
            .chars()
            .collect()
    } else {
        let alphabet_path = Path::new(alphabet_str);
        if alphabet_path.exists() {
            info!("alphabet path\t{:?}", alphabet_path);
            fs::read(&alphabet_path)
                .unwrap()
                .iter()
                .map(|&b| b as char)
                .collect()
        } else {
            info!("alphabet literal\t{:?}", alphabet_str);
            alphabet_str.chars().collect()
        }
    };
    info!("alphabet\t[{}]", alphabet.iter().collect::<String>());

    let invert = args.invert;
    info!("invert\t{}", invert);

    let font_str = &args.font;
    let font_map: HashMap<&str, &str> = FONTS.iter().cloned().collect();
    let font: font::Font = if font_map.contains_key(&font_str.as_ref()) {
        info!("font name\t{:?}", font_str);
        let font_data = font_map.get(&font_str.as_ref()).unwrap();
        Font::from_bdf_stream(font_data.as_bytes(), &alphabet, invert)
    } else {
        let font_path = Path::new(font_str);
        info!("font path\t{:?}", font_path);
        Font::from_bdf(font_path, &alphabet, invert)
    };

    let metric = args.metric;
    info!("metric\t{}", metric);

    let out_path = args.out_path.as_ref().map(|name| Path::new(name));
    info!("out path\t{:?}", out_path);

    let fps = args.fps;
    info!("fps\t{}", fps);

    let color = !args.no_color;
    info!("color\t{}", color);

    let brightness_offset = args.brightness_offset;
    info!("brightness offset\t{}", brightness_offset);

    let brightness_scale = args.brightness_scale;
    info!("brightness scale\t{}", brightness_scale);

    let edge_brightness_scale = args.edge_brightness_scale;
    info!("edge brightness scale\t{}", edge_brightness_scale);

    let naive_grayscale = args.naive_grayscale;
    info!("naive grayscale\t{}", naive_grayscale);

    let conversion_algorithm = args.conversion_algorithm;
    info!("conversion alg\t{}", conversion_algorithm);

    let convert = get_converter(&metric);
    let conversion_algorithm = get_conversion_algorithm(&conversion_algorithm);
    info!("converter\t{:?}", convert);

    info!("converting frames to ascii...");
    let frames: Vec<DynamicImage> = if in_extension == "gif" {
        let gif = gif::read_gif(image_path);
        gif.iter().cloned().collect()
    } else {
        let img = ::image::open(image_path).unwrap();
        vec![img]
    };

    let mut frame_char_rows: Vec<Vec<Vec<char>>> = Vec::new();
    let progress = default_progress_bar("Frames", frames.len());
    for img in frames.iter().progress_with(progress) {
        let grayscale_image = if naive_grayscale {
            LumaImage::naive_grayscale_from(img)
        } else {
            LumaImage::colorimetric_grayscale_from(img)
        };
        let ascii = convert::img_to_char_rows(
            &font,
            &grayscale_image,
            convert,
            width,
            brightness_offset / 255.,
            brightness_scale,
            edge_brightness_scale,
            &conversion_algorithm,
        );
        frame_char_rows.push(ascii);
    }

    if let Some(path) = out_path {
        let out_extension = path.extension().unwrap();

        if out_extension == "json" {
            let out_frames: Vec<String> = if color {
                frame_char_rows
                    .iter()
                    .zip(frames)
                    .map(|(char_rows, frame)| char_rows_to_html_color_string(char_rows, &frame))
                    .collect()
            } else {
                frame_char_rows
                    .iter()
                    .map(|char_rows| char_rows_to_string(char_rows))
                    .collect()
            };
            let json = serde_json::to_string(&out_frames).unwrap();
            fs::write(path, json).unwrap();
        } else if out_extension == "html" {
            let mut out_frames: Vec<String> = if color {
                frame_char_rows
                    .iter()
                    .zip(frames)
                    .map(|(char_rows, frame)| char_rows_to_html_color_string(char_rows, &frame))
                    .collect()
            } else {
                frame_char_rows
                    .iter()
                    .map(|char_rows| char_rows_to_string(char_rows))
                    .collect()
            };
            out_frames.insert(0, String::from("<pre>"));
            out_frames.push(String::from("</pre>"));
            fs::write(path, out_frames.join("\n")).unwrap();
        } else if out_extension == "gif" {
            info!("converting ascii strings to bitmaps...");
            let progress = default_progress_bar("Frames", frame_char_rows.len());
            let out_frames: Vec<DynamicImage> = if color {
                frame_char_rows
                    .iter()
                    .zip(frames)
                    .progress_with(progress)
                    .map(|(char_rows, frame)| char_rows_to_color_bitmap(&char_rows, &font, &frame, invert))
                    .collect()
            } else {
                frame_char_rows
                    .iter()
                    .progress_with(progress)
                    .map(|char_rows| char_rows_to_bitmap(&char_rows, &font))
                    .collect()
            };
            write_gif(path, &out_frames, fps);
        } else if out_extension == "mp4" {
            info!("converting ascii strings to bitmaps...");
            let progress = default_progress_bar("Frames", frame_char_rows.len());
            let out_frames: Vec<DynamicImage> = if color {
                frame_char_rows
                    .iter()
                    .zip(frames)
                    .progress_with(progress)
                    .map(|(char_rows, frame)| char_rows_to_color_bitmap(&char_rows, &font, &frame, invert))
                    .collect()
            } else {
                frame_char_rows
                    .iter()
                    .progress_with(progress)
                    .map(|char_rows| char_rows_to_bitmap(&char_rows, &font))
                    .collect()
            };

            let tmp_dir = temp_dir().join("image-to-ascii-frames");
            if !tmp_dir.exists() {
                fs::create_dir(tmp_dir.clone()).unwrap();
            }
            info!("writing frames to {}", tmp_dir.to_str().unwrap());
            for (i, frame) in out_frames
                .iter()
                .enumerate()
                .progress_with(default_progress_bar("Frames", out_frames.len()))
            {
                frame
                    .save(tmp_dir.join(format!("{}.png", i)))
                    .expect(&format!("Failed to write frame {}", i));
            }
            let output = std::process::Command::new("ffmpeg")
                .args([
                    "-framerate",
                    &format!("{}", fps),
                    "-i",
                    tmp_dir.join("%d.png").to_str().unwrap(),
                    "-r",
                    &format!("{}", fps),
                    path.to_str().unwrap(),
                ])
                .output();
            if let Err(err) = output {
                panic!("Error while writing mp4 frames with ffmpeg: {}", err);
            }
        } else if out_extension == "txt" {
            if color {
                warn!("color not supported with .txt output; using black and white instead");
            }
            let out_frames: Vec<String> = frame_char_rows
                .iter()
                .map(|char_rows| char_rows_to_string(char_rows))
                .collect();
            fs::write(path, out_frames.join("\n")).unwrap();
        } else {
            let img = if color {
                char_rows_to_color_bitmap(&frame_char_rows[0], &font, &frames[0], invert)
            } else {
                char_rows_to_bitmap(&frame_char_rows[0], &font)
            };
            img.save(path).unwrap();
        }
    } else {
        let out_frames: Vec<String> = if color {
            frame_char_rows
                .iter()
                .zip(frames)
                .map(|(char_rows, frame)| char_rows_to_terminal_color_string(char_rows, &frame))
                .collect()
        } else {
            frame_char_rows
                .iter()
                .map(|char_rows| char_rows_to_string(char_rows))
                .collect()
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
