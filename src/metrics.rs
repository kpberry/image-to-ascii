pub type Metric = fn(&[f32], &[f32]) -> f32;

pub fn jaccard_score(xs: &[f32], ys: &[f32]) -> f32 {
    let intersection: f32 = xs.iter().zip(ys).map(|(x, &y)| x.min(y)).sum();
    let union: f32 = xs.iter().zip(ys).map(|(x, &y)| x.max(y)).sum();
    intersection / union
}

pub fn dot_score(xs: &[f32], ys: &[f32]) -> f32 {
    xs.iter().zip(ys).map(|(x, &y)| x * y).sum()
}

pub fn occlusion_score(xs: &[f32], ys: &[f32]) -> f32 {
    let a_occlusion: f32 = xs.iter().zip(ys).map(|(x, &y)| 1. - (x - y)).sum();
    let b_occlusion = xs.iter().zip(ys).map(|(x, &y)| 1. - (y - x)).sum();
    a_occlusion.min(b_occlusion)
}

pub fn avg_color_score(xs: &[f32], ys: &[f32]) -> f32 {
    xs.iter().zip(ys).map(|(x, y)| (x + y).abs()).sum::<f32>()
}

pub fn movement_toward_clear(xs: &[f32], ys: &[f32]) -> f32 {
    -xs.iter()
        .zip(ys)
        .map(|(&x, &y)| if y > 0. { 0. } else { x })
        .sum::<f32>()
}