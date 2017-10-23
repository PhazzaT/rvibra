// Much of this module's contents was taken from ColorThief
// Or, one could say, stolen

// Data definitions

type PixelRegion<'a> = &'a mut [[u8; 3]];
// type Histogram = [usize; 65536]; // Histogram for RGB565 colors
type Color = [u8; 3];

struct Bucket<'a> {
    count: usize,
    // volume: usize,
    // bounds: [[usize; 2]; 3],
    pixels: PixelRegion<'a>,
    // histogram: &'a Histogram,
}

// Color functions

// fn to_rgb565(color: Color) -> Color {
//     let r = color[0] >> 3;
//     let g = color[1] >> 2;
//     let b = color[2] >> 3;
//     [r, g, b]
// }

// fn pack_rgb565(color: Color) -> u16 {
//     (color[0] as u16) << 11 | (color[1] as u16) << 5 | color[2] as u16
// }

// Histogram

// fn generate_histogram<'a>(pixels: PixelRegion<'a>) -> Box<Histogram> {
//     let mut histo = Box::new([0; 1 << 16]);
//     for pixel in pixels.iter() {
//         histo[pack_rgb565(to_rgb565(*pixel)) as usize] += 1;
//     }
//     histo
// }

// Bucket-related functions

fn make_bucket<'a>(pixels: PixelRegion<'a>)
        -> Bucket<'a> {
    // let bounds = {
    //     let mut bounds = [[0; 2]; 3];
    //     for i in 0..3 {
    //         let min = pixels.iter().map(|c| c[i]).min().unwrap();
    //         let max = pixels.iter().map(|c| c[i]).max().unwrap();
    //         bounds[i] = [min as usize, (max + 1) as usize]
    //     }
    //     bounds
    // };
    // let count = {
    //     let mut count = 0;
    //     for r in bounds[0][0]..bounds[0][1] {
    //         for g in bounds[1][0]..bounds[1][1] {
    //             for b in bounds[2][0]..bounds[2][1] {
    //                 let packed = pack_rgb565([r as u8, g as u8, b as u8]);
    //                 count += histogram[packed as usize];
    //             }
    //         }
    //     }
    //     count
    // };
    // let volume = bounds.iter().map(|d| d[1] - d[0]).product();
    Bucket {
        count: pixels.len(),
        // volume: volume,
        // bounds: bounds,
        pixels: pixels,
        // histogram: histogram,
    }
}

fn split_bucket<'a>(bucket: Bucket<'a>)
        -> (Option<Bucket<'a>>, Option<Bucket<'a>>) {
    let pixels = bucket.pixels;

    // Find the widest axis
    let axis = {
        let mut best_axis = 0;
        let mut best_width = 0;
        let shifts: [usize; 3] = [5, 6, 5];
        for (i, shift) in (0..3).into_iter().zip(shifts.into_iter()) {
            let min = pixels.iter().map(|c| c[i] >> shift).min().unwrap();
            let max = pixels.iter().map(|c| c[i] >> shift).max().unwrap();
            let new_width = max - min;
            if best_width < new_width {
                best_width = new_width;
                best_axis = i;
            }
        }
        best_axis
    };

    // Sort and split along the axis
    pixels.sort_unstable_by_key(|c| c[axis]);
    let split_position = pixels.len() / 2;
    let (low, high) = pixels.split_at_mut(split_position);

    ( Some(make_bucket(low))
    , Some(make_bucket(high)) )
}

fn color_from_bucket<'a>(bucket: Bucket<'a>) -> Color {
    let mut ret = [0, 0, 0];
    for c in bucket.pixels.iter() {
        for i in 0..3 {
            ret[i] += c[i] as u64;
        }
    }
    for i in 0..3 {
        ret[i] /= bucket.pixels.len() as u64;
    }
    [ret[0] as u8, ret[1] as u8, ret[2] as u8]
}

pub fn quantize<'a>(pixels: PixelRegion<'a>, max_color_count: usize)
        -> Vec<Color> {
    assert!(pixels.len() > 0);
    assert!(max_color_count >= 2 && max_color_count <= 256);

    // let histo = generate_histogram(pixels);
    let mut queue = vec![make_bucket(pixels)];

    {
        let mut iter = |f: fn(&Bucket) -> usize, target: usize| -> usize {
            let mut n_color = 1;
            for _ in 0..1000 {
                if n_color >= target {
                    break;
                }
                queue.sort_unstable_by_key(f);
                let b = queue.pop().unwrap();
                if b.count == 0 {
                    queue.push(b);
                    continue;
                }
                n_color -= 1;
                let (b0, b1) = split_bucket(b);
                for b in b0.into_iter().chain(b1.into_iter()) {
                    queue.push(b);
                    n_color += 1;
                }
            }
            queue.len()
        };
        iter(|b| b.count, max_color_count);
    }
    // let qlen = iter(|b| b.count, max_color_count * 3 / 4);
    // iter(|b| b.count * b.volume, max_color_count - qlen);

    queue.into_iter().map(color_from_bucket).collect()
}
