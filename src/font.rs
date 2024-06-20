use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Clone)]
pub struct Character {
    pub value: char,
    pub bitmap: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl Character {
    pub fn new(value: char, bitmap: Vec<f32>, width: usize, height: usize) -> Character {
        Character {
            value,
            bitmap,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.bitmap[y * self.width + x]
    }
}

#[derive(Clone)]
pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
    pub intensities: Vec<f32>,
    pub grads: Vec<(f32, f32)>,
    pub char_map: HashMap<char, Character>,
    pub intensity_chars: Vec<Character>,
}

impl Font {
    pub fn new(chars: &[Character], alphabet: &[char]) -> Font {
        let char_set: HashSet<char> = alphabet.iter().cloned().collect();
        let chars: Vec<Character> = chars
            .iter()
            .filter(|c| char_set.contains(&c.value))
            .cloned()
            .collect();

        let min_height = chars.iter().map(|c| c.height).min().unwrap();
        let max_height = chars.iter().map(|c| c.height).max().unwrap();
        if max_height != min_height {
            panic!(
                "All Characters must have the same height; found values between {} and {}",
                min_height, max_height
            )
        }

        let min_width = chars.iter().map(|c| c.width).min().unwrap();
        let max_width = chars.iter().map(|c| c.width).max().unwrap();
        if max_width != min_width {
            panic!(
                "All Characters must have the same width; found values between {} and {}",
                min_width, max_width
            )
        }

        let (width, height) = (min_width, min_height);

        let intensity_indexes: Vec<i32> = chars
            .iter()
            .cloned()
            .map(|c| c.bitmap.iter().sum::<f32>() as i32)
            .collect();
        let max_intensity = *intensity_indexes.iter().max().unwrap_or(&0);
        let max_possible_intensity = (width * height) as i32;
        let intensities: Vec<i32> = intensity_indexes
            .iter()
            .map(|intensity| (intensity * max_possible_intensity / max_intensity) as i32)
            .collect();

        let mut char_intensities: Vec<(i32, Character)> = intensities
            .iter()
            .cloned()
            .zip(chars.iter().cloned())
            .collect();
        char_intensities.sort_by_key(|(intensity, _)| *intensity);
        let mut intensity_chars: Vec<Character> =
            Vec::with_capacity(max_possible_intensity as usize + 1);
        let mut index = 0;
        for i in 0..=max_possible_intensity {
            while i > char_intensities[index].0 {
                index += 1;
            }
            intensity_chars.push(char_intensities[index].1.clone());
        }

        let intensities: Vec<f32> = chars
            .iter()
            .cloned()
            .map(|c| c.bitmap.iter().sum::<f32>())
            .collect();

        let grads: Vec<(f32, f32)> = chars
            .iter()
            .cloned()
            .map(|c| {
                let mut x_grad = 0.0;
                let mut y_grad = 0.0;
                for i in 0..height {
                    for j in 0..width - 1 {
                        if c.bitmap[i * width + 1 + j] > c.bitmap[i * width + j] {
                            x_grad += 1.;
                        }
                    }
                }
                for i in 0..height - 1 {
                    for j in 0..width {
                        if c.bitmap[(i + 1) * width + j] > c.bitmap[i * width + j] {
                            y_grad += 1.;
                        }
                    }
                }
                (x_grad, y_grad)
            })
            .collect();

        let char_map = chars.iter().map(|c| (c.value, c.clone())).collect();

        Font {
            width,
            height,
            chars,
            intensities,
            grads,
            char_map,
            intensity_chars,
        }
    }

    pub fn from_bdf_stream<R: Read>(stream: R, alphabet: &[char]) -> Font {
        let buf_reader = BufReader::new(stream);
        let font: bdf_reader::Font = bdf_reader::Font::read(buf_reader).unwrap();
        let mut chars: Vec<Character> = font
            .glyphs()
            .into_iter()
            .map(|glyph| {
                let value: char = glyph.encoding() as u8 as char;
                let bbox = glyph.bounding_box();
                let width = bbox.width as usize;
                let height = bbox.height as usize;
                let glyph_bitmap = glyph.bitmap();
                let mut bitmap = Vec::new();
                for y in 0..height {
                    for x in 0..width {
                        bitmap.push(if glyph_bitmap.get(x, y).unwrap() {
                            1.
                        } else {
                            0.
                        });
                    }
                }
                Character::new(value, bitmap, width, height)
            })
            .collect();
        chars.sort_by_key(|c| c.value as u8);

        Font::new(&chars, alphabet)
    }

    pub fn from_bdf(path: &Path, alphabet: &[char]) -> Font {
        Font::from_bdf_stream(File::open(path).unwrap(), alphabet)
    }

    pub fn _print(&self) {
        for c in &self.chars {
            println!("{}", c.value);
            for y in 0..c.height {
                for x in 0..c.width {
                    if c.get(x, y) > 0. {
                        print!("██");
                    } else {
                        print!("  ");
                    }
                }
                print!("\n");
            }
        }
    }
}
