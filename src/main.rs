extern crate image;

use std::env::args_os;
use std::ffi::OsString;
use std::path::Path;

mod processing;

// Some functionality is based on ColorThief's behavior

fn load_pixels<'a>(path: &OsString) -> Box<[[u8; 3]]> {
    image::open(&Path::new(path))
        .unwrap()
        .to_rgb()
        .pixels()
        .filter(|px| px[0] <= 250 || px[1] <= 250 || px[2] <= 250)
        .map(|px| [px[0], px[1], px[2]])
        .collect::<Vec<[u8; 3]>>()
        .into_boxed_slice()
}

fn main() {
    let mut args = args_os();
    args.next();
    let path = args.next().unwrap();

    let color_count = args.next()
        .unwrap().to_str()
        .unwrap().parse::<usize>()
        .unwrap();
    let mut img = load_pixels(&path);
    let colors = processing::quantize(&mut img, color_count);

    for color in colors.iter() {
        println!("{} {} {}", color[0], color[1], color[2]);
    }
}
