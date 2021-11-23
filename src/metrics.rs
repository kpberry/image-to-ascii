fn jaccard_score(xs: &[f32], ys: &[f32]) -> f32 {
    let intersection: f32 = xs.iter().zip(ys).map(|(x, &y)| x.min(y)).sum();
    let union: f32 = xs.iter().zip(ys).map(|(x, &y)| x.max(y)).sum();
    intersection / union
}

fn dot_score(xs: &[f32], ys: &[f32]) -> f32 {
    xs.iter().zip(ys).map(|(x, &y)| x * y).sum()
}

fn occlusion_score(xs: &[f32], ys: &[f32]) -> f32 {
    let a_occlusion: f32 = xs.iter().zip(ys).map(|(x, &y)| 1. - (x - y)).sum();
    let b_occlusion = xs.iter().zip(ys).map(|(x, &y)| 1. - (y - x)).sum();
    a_occlusion.min(b_occlusion)
}

fn avg_color_score(xs: &[f32], ys: &[f32]) -> f32 {
    let len = xs.len() as f32;
    (len - (xs.iter().sum::<f32>() - ys.iter().sum::<f32>()).abs()) / len
}

fn movement_toward_clear(xs: &[f32], ys: &[f32]) -> f32 {
    let len = xs.len() as f32;
    (len - xs.iter().zip(ys).map(|(&x, &y)| if y > 0. { 0. } else { x }).sum::<f32>()) / (len - xs.iter().sum::<f32>())
}