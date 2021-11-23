use std::path::Path;

pub struct Character {
    pub value: char,
    pub bitmap: Vec<f32>,
    pub width: usize,
    pub height: usize
}

impl Character {
    pub fn new(value: char, bitmap: Vec<f32>, width: usize, height: usize) -> Character {
        Character { value, bitmap, width, height }
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.bitmap[y * self.width + x]
    }
}

pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
}

impl Font {
    pub fn new(chars: Vec<Character>) -> Font {
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

        Font {
            width: min_width,
            height: min_height,
            chars,
        }
    }

    pub fn from_bdf(path: &Path) -> Font {
        let font: bdf::Font = bdf::open(path).unwrap();
        let mut chars: Vec<Character> = font.glyphs().iter().map(
            |(character, glyph)| {
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
            }
        ).collect();
        chars.sort_by_key(|c| c.value as u8);

        Font::new(chars)
    }

    pub fn print(&self) {
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
