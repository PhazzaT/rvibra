use float_ord::*;
use rayon::prelude::*;
use rayon;

type PixelRegion<'a> = &'a mut [[u8; 3]];
type Color = [u8; 3];
type Center = [f64; 3];

#[derive(Debug, Clone)]
struct Cluster {
    centroid: Center,
    mean: Center,
    size: usize,
}

fn sqr(x: f64) -> f64 {
    x * x
}

fn distance(fc: Center, c: Color) -> FloatOrd<f64> {
    FloatOrd(sqr(fc[0] - c[0] as f64)
            + sqr(fc[1] - c[1] as f64)
            + sqr(fc[2] - c[2] as f64))
}

fn center_distance(c1: Center, c2: Center) -> FloatOrd<f64> {
    FloatOrd(sqr(c1[0] - c2[0])
            + sqr(c1[1] - c2[1])
            + sqr(c1[2] - c2[2]))
}

fn initialize<'a>(pixels: PixelRegion<'a>, max_color_count: usize)
        -> Vec<Center> {
    // K-means++, but instead of randomly choosing,
    // take the most distant color in each iteration
    let mut centers = Vec::with_capacity(max_color_count);

    eprintln!("Choosing starting center 0...");
    centers.push({
        let mut acc = pixels.par_iter()
            .map(|px| [px[0] as f64, px[1] as f64, px[2] as f64])
            .reduce(|| [0f64; 3], |a, b| {
                [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
            });
        for i in 0..3 { acc[i] /= max_color_count as f64; }
        acc
    });

    for i in 1..max_color_count {
        eprintln!("Choosing starting center {}...", i);
        let new = pixels.par_iter().max_by_key(|px| {
            centers.iter().map(|c| distance(*c, **px)).max()
        }).unwrap();
        centers.push([new[0] as f64, new[1] as f64, new[2] as f64]);
    }

    centers
}

fn compute_centroids(mut clusters: Vec<Cluster>, slice: &[[u8; 3]])
        -> Vec<Cluster> {
    
    const SEQUENTIAL_BLOCK_SIZE: usize = 1024;
    if slice.len() <= SEQUENTIAL_BLOCK_SIZE {
        for px in slice {
            let closest = clusters.iter_mut().min_by_key(|c| {
                distance(c.mean, *px)
            }).unwrap();
            for i in 0..3 {
                closest.centroid[i] += px[i] as f64;
            }
            closest.size += 1;
        }
    } else {
        let (sa, sb) = slice.split_at(slice.len() / 2);
        let (ca, cb) = rayon::join(
            || compute_centroids(clusters.clone(), sa),
            || compute_centroids(clusters.clone(), sb)
        );
        for i in 0..clusters.len() {
            for j in 0..3 {
                clusters[i].centroid[j] = ca[i].centroid[j] + cb[i].centroid[j];
            }
            clusters[i].size = ca[i].size + cb[i].size;
        }
    }
    clusters
}

pub fn quantize<'a>(pixels: PixelRegion<'a>, max_color_count: usize)
        -> Vec<Color> {
    // Lloyd's algorithm
    let mut clusters = initialize(pixels, max_color_count).into_iter().map(|c| {
        Cluster {
            centroid: [0.0, 0.0, 0.0],
            mean: c,
            size: 0
        }
    }).collect::<Vec<_>>();

    let mut difference: Option<f64> = None;
    let mut iterations = 0;
    while difference.map_or(true, |d| d != 0f64) {
        // Compute centroids
        clusters = compute_centroids(clusters, pixels);

        // Move means to centroids
        let mut biggest_movement = 0.0;
        for cluster in clusters.iter_mut() {
            if cluster.size != 0 {
                for i in 0..3 {
                    cluster.centroid[i] /= cluster.size as f64;
                }
                let movement = center_distance(cluster.mean, cluster.centroid);
                cluster.mean = cluster.centroid;
                cluster.centroid = [0.0, 0.0, 0.0];
                cluster.size = 0;

                if biggest_movement < movement.0 {
                    biggest_movement = movement.0;
                }
            }
        }

        difference = Some(biggest_movement);
        iterations += 1;
        eprintln!("Step {}, difference: {}", iterations, biggest_movement);
    }

    clusters.into_iter().map(|c| {
        [c.mean[0] as u8, c.mean[1] as u8, c.mean[2] as u8]
    }).collect::<Vec<_>>()
}
