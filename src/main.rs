extern crate image;

use std::env::args_os;
use std::ffi::OsString;
use std::path::Path;
use image::RgbImage;

// mod processing;
mod kmeans;
mod float_ord;

// Some functionality is based on ColorThief's behavior

fn load_pixels<'a>(img: &'a RgbImage) -> Box<[[u8; 3]]> {
    img
        .pixels()
        .filter(|px| px[0] <= 250 || px[1] <= 250 || px[2] <= 250)
        .filter(|px| px[0] >= 5 || px[1] >= 5 || px[2] >= 5)
        .map(|px| [px[0], px[1], px[2]])
        .collect::<Vec<[u8; 3]>>()
        .into_boxed_slice()
}

fn posterize<'a>(img: &'a mut RgbImage, palette: &'a Vec<[u8; 3]>) {
    let sqr = |x: i32| x * x;
    for (_, _, px) in img.enumerate_pixels_mut() {
        // Find closest color from palette
        let closest = palette.iter().min_by_key(|c| {
            (0..3).map(|i| sqr(c[i] as i32 - px[i] as i32)).sum::<i32>()
        }).unwrap();
        for i in 0..3 { px[i] = closest[i]; }
    }
}

fn main() {
    let mut args = args_os();
    args.next();
    let path = args.next().unwrap();

    let color_count = args.next()
        .unwrap().to_str()
        .unwrap().parse::<usize>()
        .unwrap();

    let mut img = image::open(&Path::new(&path)).unwrap().to_rgb();
    let mut pixels = load_pixels(&img);
    let colors = kmeans::quantize(&mut pixels, color_count);

    for color in colors.iter() {
        println!("{} {} {}", color[0], color[1], color[2]);
    }

    posterize(&mut img, &colors);
    let new_path = String::from(path.to_string_lossy()) + ".posterized.png";
    img.save(OsString::from(new_path)).unwrap();
}
