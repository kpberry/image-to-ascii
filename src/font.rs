use std::collections::HashSet;
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

        let intensities: Vec<i32> = chars
            .iter()
            .cloned()
            .map(|c| c.bitmap.iter().sum::<f32>() as i32)
            .collect();
        let max_intensity = *intensities.iter().max().unwrap_or(&0);
        let max_possible_intensity = (width * height) as i32;
        let intensities: Vec<i32> = intensities
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

        Font {
            width,
            height,
            chars,
            intensity_chars,
        }
    }

    pub fn from_bdf(path: &Path, alphabet: &[char]) -> Font {
        let font: bdf::Font = bdf::open(path).unwrap();
        let mut chars: Vec<Character> = font
            .glyphs()
            .iter()
            .map(|(character, glyph)| {
                let value = *character;
                let width = glyph.width() as usize;
                let height = glyph.height() as usize;
                let mut bitmap = Vec::new();
                for y in 0..(height as u32) {
                    for x in 0..(width as u32) {
                        bitmap.push(if glyph.get(x, y) { 1. } else { 0. });
                    }
                }
                Character::new(value, bitmap, width, height)
            })
            .collect();
        chars.sort_by_key(|c| c.value as u8);

        Font::new(&chars, alphabet)
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
