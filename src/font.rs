pub struct Character {
    pub value: u8,
    pub glyph: Vec<Vec<u8>>,
}

impl Character {
    fn new(value: u8, data: Vec<Vec<u8>>) -> Character {
        Character { value, glyph: data }
    }
    fn width(&self) -> usize {
        self.glyph[0].len()
    }

    fn height(&self) -> usize {
        self.glyph.len()
    }
}

pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
}

impl Font {
    pub fn new(chars: Vec<Character>) -> Font {
        let min_height = chars.iter().map(|c| c.height()).min().unwrap();
        let max_height = chars.iter().map(|c| c.height()).max().unwrap();
        if max_height != min_height {
            panic!(
                "All Characters must have the same height; found values between {} and {}",
                min_height, max_height
            )
        }

        let min_width = chars.iter().map(|c| c.width()).min().unwrap();
        let max_width = chars.iter().map(|c| c.width()).max().unwrap();
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
}
