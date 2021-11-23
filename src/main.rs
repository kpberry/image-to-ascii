use std::collections::HashSet;
use std::env;
use std::path::Path;
use crate::font::Font;

mod font;
mod metrics;
mod gif;
mod convert;

fn main() {
    let mut font = Font::from_bdf(Path::new("fonts/kourier.bdf"));
    let alphabet = vec![
        ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')',
        '*', '+', ',', '-', '.', '/', '0', '1', '2', '3',
        '4', '5', '6', '7', '8', '9', ':', ';', '=', '?',
        '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
        'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
        'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']',
        '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
        'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
        'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{',
        '|', '}', '~'
    ];
    let char_set: HashSet<char> = alphabet.iter().cloned().collect();
    font.chars = font.chars.iter().filter(|c| char_set.contains(&c.value)).cloned().collect();

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let width = args[2].parse::<usize>().unwrap_or(150);

    let path = Path::new(filename);
    let extension = path.extension().unwrap();

    if extension == "gif" {
        let gif = gif::read_gif(path);
        for img in gif {
            let ascii = convert::img_to_ascii(&font, &img, width);
            println!("{}[2J{}", 27 as char, ascii);
        }
    } else {
        let img = image::open(path).unwrap();
        let ascii = convert::img_to_ascii(&font, &img, width);
        println!("{}", ascii);
    }
}