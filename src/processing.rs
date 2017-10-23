// Much of this module's contents was taken from ColorThief
// Or, one could say, stolen
// Heh

use std::ops::Range;

// Data definitions

type PixelRegion<'a> = &'a mut [[u8; 3]];
type Histogram = [usize; 32768]; // Histogram for RGB565 colors
type Color = [u8; 3];

struct Bucket<'a> {
    volume: usize,
    bounds: [Range<usize>; 3],
    pixels: PixelRegion<'a>,
}

// Utility

// Partitions by given predicate. Elements which evaluate
// to false are put before those which evaluate to true.
// Returns the size of the "false" range.
fn partition_by<'a, T, F>(arr: &'a mut [T], f: F) -> usize
        where F: Fn(&T) -> bool {
    let mut begin = 0;
    let mut end = 0;
    while end < arr.len() {
        if !f(&arr[end]) {
            arr.swap(begin, end);
            begin += 1;
        }
        end += 1;
    }
    begin
}

// Color functions

fn to_rgb555(color: Color) -> Color {
    let r = color[0] >> 3;
    let g = color[1] >> 3;
    let b = color[2] >> 3;
    [r, g, b]
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

fn make_bucket<'a>(pixels: PixelRegion<'a>)
        -> Bucket<'a> {
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
        bounds: bounds,
        pixels: pixels,
    }
}

fn split_bucket<'a>(mut bucket: Bucket<'a>, histogram: &'a Histogram)
        -> (Option<Bucket<'a>>, Option<Bucket<'a>>) {
    if bucket.pixels.len() == 1 {
        return (Some(bucket), None);
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

    // Figure out how to partition buckets, so as to minimize
    // new buckets' size difference
    let mut prefix_sums = [0; 1 << 5];
    for i in 1..1 << 5 {
        prefix_sums[i] = prefix_sums[i - 1] + buckets[i - 1];
    }
    let best_split = (0..1 << 5).into_iter().min_by_key(|i| {
        (2 * prefix_sums[*i] as i64 - bucket.pixels.len() as i64).abs()
    }).unwrap();

    // println!("{:?}", prefix_sums);
    // println!("best_split: {}", best_split);
    // println!("bucket.bounds: {:?}", bucket.bounds);
    // println!("buckets: {:?}", buckets);
    // println!("prefix_sum: {:?}", prefix_sums);
    // println!("{:?}", (0..1 << 5).into_iter().map(|i| {
    //     (2 * prefix_sums[i] as i64 - bucket.pixels.len() as i64).abs()
    // }).collect::<Vec<_>>());
    // println!("bucket sum: {}", buckets.iter().sum::<usize>());
    // println!("bucket.pixels.len(): {}", bucket.pixels.len());
    // assert!(buckets.iter().sum::<usize>() == bucket.pixels.len());

    if prefix_sums[best_split] == 0
            || prefix_sums[best_split] == bucket.pixels.len() {
        // Can't split into two non-empty buckets
        return (Some(bucket), None);
    }

    // Partition into two subranges
    let split_position = partition_by(&mut bucket.pixels, |c| { 
        (c[x] >> 3) < best_split as u8
    });
    let (low, high) = bucket.pixels.split_at_mut(split_position);
    // println!("split_position: {}", split_position);

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

    let histogram = generate_histogram(pixels);
    let mut queue = vec![make_bucket(pixels)];

    {
        let mut iter = |f: fn(&Bucket) -> usize, target: usize| {
            let mut n_color = queue.len();
            for _ in 0..1000 {
                if n_color >= target {
                    break;
                }
                queue.sort_unstable_by_key(f);
                let b = queue.pop().unwrap();
                if b.pixels.len() == 0 {
                    queue.push(b);
                    continue;
                }
                n_color -= 1;
                let (b0, b1) = split_bucket(b, &histogram);
                for b in b0.into_iter().chain(b1.into_iter()) {
                    queue.push(b);
                    n_color += 1;
                }
            }
        };
        // iter(|b| b.pixels.len(), max_color_count);
        iter(|b| b.pixels.len(), max_color_count * 3 / 4);
        iter(|b| b.pixels.len() * b.volume, max_color_count);
    }

    queue.into_iter().map(color_from_bucket).collect()
}
