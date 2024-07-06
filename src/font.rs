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
    pub intensity: f32,
    pub grad: (f32, f32),
    pub direction: (f32, f32),
}

impl Character {
    pub fn new(value: char, bitmap: Vec<f32>, width: usize, height: usize) -> Character {
        let intensity = bitmap.iter().sum::<f32>();
        let grad = Character::grad(&bitmap, width, height);
        let direction = Character::direction(&bitmap, width, height);

        Character {
            value,
            bitmap,
            width,
            height,
            intensity,
            grad,
            direction,
        }
    }

    fn grad(bitmap: &[f32], width: usize, height: usize) -> (f32, f32) {
        let mut x_grad = 0.0;
        let mut y_grad = 0.0;
        for i in 0..height {
            for j in 0..width - 1 {
                if bitmap[i * width + 1 + j] > bitmap[i * width + j] {
                    x_grad += 1.;
                }
            }
        }
        for i in 0..height - 1 {
            for j in 0..width {
                if bitmap[(i + 1) * width + j] > bitmap[i * width + j] {
                    y_grad += 1.;
                }
            }
        }
        (x_grad, y_grad)
    }

    fn direction(bitmap: &[f32], width: usize, height: usize) -> (f32, f32) {
        // computes approximately which "direction" a bitmap is facing
        //
        // uses some special kernels to compute a one-sided "center of mass" for
        // each non-zero entry in the bitmap, which is is a local indicator for
        // which direction that part of the bitmap is "facing", then averages
        // them
        let mut grid = Vec::new();
        let mut i = 0;
        for _ in 0..height {
            let mut row = Vec::new();
            for _ in 0..width {
                row.push(bitmap[i]);
                i += 1;
            }
            grid.push(row);
        }

        let total_kernel = [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0]]
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();
        let total = sum_2d(&masked_discrete_convolution_2d(&grid, &total_kernel));
        if total == 0.0 {
            return (0.0, 0.0);
        }

        let x_kernel = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 1.0]]
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();
        let y_kernel = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();

        (
            sum_2d(&masked_discrete_convolution_2d(&grid, &x_kernel)) / total,
            sum_2d(&masked_discrete_convolution_2d(&grid, &y_kernel)) / total,
        )
    }

    #[allow(dead_code)]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.bitmap[y * self.width + x]
    }
}

fn masked_discrete_convolution_2d(grid: &Vec<Vec<f32>>, kernel: &Vec<Vec<f32>>) -> Vec<Vec<f32>> {
    // offset kernel to keep output size the same
    let kernel_row_offset = ((kernel.len() - 1) / 2) as i64;
    let kernel_col_offset = ((kernel[0].len() - 1) / 2) as i64;

    let (data_height, data_width) = (grid.len() as i64, grid[0].len() as i64);

    let mut result = Vec::new();
    for r in 0..data_height {
        let mut row = Vec::new();
        for c in 0..data_width {
            // this masking means we only update the total for nonzero values in data, as opposed
            // to a regular convolution which would update the total for all values in data
            if grid[r as usize][c as usize] == 0.0 {
                continue;
            }

            let mut total = 0.0;
            for kr in 0..(kernel.len() as i64) {
                for kc in 0..(kernel[0].len() as i64) {
                    let dr = r + kr - kernel_row_offset;
                    let dc = c + kc - kernel_col_offset;

                    if dr >= 0 && dr < data_height && dc >= 0 && dc < data_width {
                        total += kernel[kr as usize][kc as usize] * grid[dr as usize][dc as usize];
                    }
                }
            }
            row.push(total);
        }
        result.push(row);
    }

    result
}

fn sum_2d(grid: &Vec<Vec<f32>>) -> f32 {
    grid.iter().map(|row| row.iter().sum::<f32>()).sum()
}

#[derive(Clone)]
pub struct Font {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Character>,
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

        let max_intensity = chars.iter().map(|c| c.intensity as i32).max().unwrap_or(0);
        let max_possible_intensity = (width * height) as i32;
        let normalized_intensities: Vec<i32> = chars
            .iter()
            .map(|c| (c.intensity as i32 * max_possible_intensity / max_intensity) as i32)
            .collect();

        let mut char_intensities: Vec<(i32, Character)> = normalized_intensities
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

        let char_map = chars.iter().map(|c| (c.value, c.clone())).collect();

        Font {
            width,
            height,
            chars,
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
