use iter_utils::*;

type PixelRegion<'a> = &'a mut [[u8; 3]];
type Color = [u8; 3];
type Center = [f64; 3];

#[derive(Debug)]
struct Cluster {
    centroid: Center,
    mean: Center,
    size: usize,
}

fn sqr(x: f64) -> f64 {
    x * x
}

fn distance(fc: Center, c: Color) -> f64 {
    sqr(fc[0] - c[0] as f64)
    + sqr(fc[1] - c[1] as f64)
    + sqr(fc[2] - c[2] as f64)
}

fn center_distance(c1: Center, c2: Center) -> f64 {
    sqr(c1[0] - c2[0])
    + sqr(c1[1] - c2[1])
    + sqr(c1[2] - c2[2])
}

fn initialize<'a>(pixels: PixelRegion<'a>, max_color_count: usize)
        -> Vec<Center> {
    // K-means++, but instead of randomly choosing,
    // take the most distant color in each iteration
    let mut centers = Vec::with_capacity(max_color_count);

    eprintln!("Choosing starting center 0...");
    centers.push({
        let mut acc = [0f64; 3];
        for px in pixels.iter() {
            for i in 0..3 { acc[i] += px[i] as f64; }
        }
        for i in 0..3 { acc[i] /= max_color_count as f64; }
        acc
    });

    for i in 1..max_color_count {
        eprintln!("Choosing starting center {}...", i);
        let new = max_by_key_partial(pixels.iter(), |px| {
            max_partial(centers.iter().map(|c| distance(*c, **px)))
        }).unwrap();
        centers.push([new[0] as f64, new[1] as f64, new[2] as f64]);
    }

    centers
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
        for px in pixels.iter() {
            // Find the closest cluster
            let closest = min_by_key_partial(clusters.iter_mut(), |c| {
                distance(c.mean, *px)
            }).unwrap();
            for i in 0..3 {
                closest.centroid[i] += px[i] as f64;
            }
            closest.size += 1;
        }

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

                if biggest_movement < movement {
                    biggest_movement = movement;
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
