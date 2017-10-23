// Much of this module's contents was taken from ColorThief
// Or, one could say, stolen
// Heh

use std::ops::Range;

// Data definitions

type PixelRegion<'a> = &'a mut [[u8; 3]];
type Histogram = [usize; 32768]; // Histogram for RGB565 colors
type Color = [u8; 3];

#[derive(Clone)]
struct Bucket {
    volume: usize,
    weight: usize,
    bounds: [Range<usize>; 3],
}

// Utility

// Color functions

fn to_rgb555(color: Color) -> Color {
    let r = color[0] >> 3;
    let g = color[1] >> 3;
    let b = color[2] >> 3;
    [r, g, b]
}

fn to_rgb888(color: Color) -> Color {
    let promote = |x| (x << 3) | (1 << 2) as u8;
    [promote(color[0]), promote(color[1]), promote(color[2])]
}

fn pack_rgb555(color: Color) -> u16 {
    (color[0] as u16) << 10 | (color[1] as u16) << 5 | color[2] as u16
}

// Histogram

fn generate_histogram<'a>(pixels: PixelRegion<'a>) -> Box<Histogram> {
    let mut histo = Box::new([0; 1 << 15]);
    for pixel in pixels.iter() {
        histo[pack_rgb555(to_rgb555(*pixel)) as usize] += 1;
    }
    histo
}

// Bucket-related functions

fn bucket_weight(bounds: &[Range<usize>; 3], histogram: &Histogram) -> usize {
    let mut sum = 0;
    for i in bounds[0].clone() {
        for j in bounds[1].clone() {
            for k in bounds[2].clone() {
                let p = pack_rgb555([i as u8, j as u8, k as u8]);
                sum += histogram[p as usize];
            }
        }
    }
    sum
}

fn make_bucket_from_pixels<'a>(pixels: PixelRegion<'a>)
        -> Bucket {
    let bounds = {
        let mut bounds = [0..0, 0..0, 0..0];
        for i in 0..3 {
            let min = pixels.iter().map(|c| c[i] >> 3).min().unwrap();
            let max = pixels.iter().map(|c| c[i] >> 3).max().unwrap();
            bounds[i] = min as usize .. (max + 1) as usize
        }
        bounds
    };
    let volume = bounds.iter().map(|d| d.len()).product();
    Bucket {
        volume: volume,
        weight: pixels.len(),
        bounds: bounds,
    }
}

fn split_bucket_along(bucket: Bucket, axis: usize, split_position: usize,
        histogram: &Histogram) -> (Bucket, Bucket) {
    let mut b0 = bucket;
    let mut b1 = b0.clone();

    b0.bounds[axis].end = split_position;
    b0.weight = bucket_weight(&b0.bounds, histogram);
    b1.bounds[axis].start = split_position;
    b1.weight = bucket_weight(&b1.bounds, histogram);

    (b0, b1)
}

fn split_bucket<'a>(bucket: Bucket, histogram: &'a Histogram)
        -> (Bucket, Option<Bucket>) {
    if bucket.volume == 1 {
        return (bucket, None);
    }

    // Find the widest axis, and the traversal order
    let (x, y, z) = {
        let bounds = bucket.bounds.clone();
        let dx = bounds[0].len();
        let dy = bounds[1].len();
        let dz = bounds[2].len();
        let max = *[dx, dy, dz].into_iter().max().unwrap();
        if dx == max      { (0, 1, 2) }
        else if dy == max { (1, 0, 2) }
        else              { (2, 0, 1) }
    };

    // Generate mini-histogram for given reduced dimension
    let mut buckets = [0; 1 << 5];
    for i in bucket.bounds[x].clone() {
        let mut px = [0, 0, 0];
        px[x] = i as u8;
        let mut sum = 0;
        for j in bucket.bounds[y].clone() {
            px[y] = j as u8;
            for k in bucket.bounds[z].clone() {
                px[z] = k as u8;
                let p = pack_rgb555(px);
                sum += histogram[p as usize];
            }
        }
        buckets[i] = sum;
    }
    let pixel_sum = buckets.iter().sum();

    // Figure out how to partition buckets, so as to minimize
    // new buckets' size difference
    let mut prefix_sums = [0; 1 << 5];
    for i in 1..1 << 5 {
        prefix_sums[i] = prefix_sums[i - 1] + buckets[i - 1];
    }
    let best_split = (0..1 << 5).into_iter().min_by_key(|i| {
        (2 * prefix_sums[*i] as i64 - pixel_sum as i64).abs()
    }).unwrap();

    if prefix_sums[best_split] == 0
            || prefix_sums[best_split] == pixel_sum {
        // Can't split into two non-empty buckets
        return (bucket, None);
    }

    // Partition into two subranges
    let (low, high) = split_bucket_along(bucket, x, best_split, histogram);
    (low, Some(high))
}

fn color_from_bucket(bucket: Bucket, histogram: &Histogram) -> Color {
    let mut avg = [0, 0, 0];
    for i in bucket.bounds[0].clone() {
        for j in bucket.bounds[1].clone() {
            for k in bucket.bounds[2].clone() {
                let p = pack_rgb555([i as u8, j as u8, k as u8]);
                let mult = histogram[p as usize];
                avg[0] += i * mult;
                avg[1] += j * mult;
                avg[2] += k * mult;
            }
        }
    }

    avg[0] /= bucket.weight;
    avg[1] /= bucket.weight;
    avg[2] /= bucket.weight;
    to_rgb888([avg[0] as u8, avg[1] as u8, avg[2] as u8])
}

pub fn quantize<'a>(pixels: PixelRegion<'a>, max_color_count: usize)
        -> Vec<Color> {
    assert!(pixels.len() > 0);
    assert!(max_color_count >= 2 && max_color_count <= 256);

    let histogram = generate_histogram(pixels);
    let mut queue = vec![make_bucket_from_pixels(pixels)];

    {
        let mut iter = |f: fn(&Bucket) -> usize, target: usize| {
            let mut n_color = queue.len();
            for _ in 0..1000 {
                if n_color >= target {
                    break;
                }
                queue.sort_unstable_by_key(f);
                let b = queue.pop().unwrap();
                let (b0, b1) = split_bucket(b, &histogram);
                queue.push(b0);
                b1.map(|b| {
                    queue.push(b);
                    n_color += 1;
                });
            }
        };
        // iter(|b| b.pixels.len(), max_color_count);
        iter(|b| b.weight, max_color_count * 3 / 4);
        iter(|b| b.weight * b.volume, max_color_count);
    }

    queue.into_iter().map(|b| color_from_bucket(b, &histogram)).collect()
}
