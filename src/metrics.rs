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
    -xs.iter().zip(ys).map(|(x, y)| (x + y).abs()).sum::<f32>()
}

pub fn movement_toward_clear(xs: &[f32], ys: &[f32]) -> f32 {
    -xs.iter()
        .zip(ys)
        .map(|(&x, &y)| if y > 0. { 0. } else { x })
        .sum::<f32>()
}

pub fn grad_and_intensity_score(xs: &[f32], ys: &[f32]) -> f32 {
    // TODO sizes are hardcoded since the other metrics don't take width/height info
    let mut xs_rect = Vec::new();
    for i in (0..xs.len()).step_by(7) {
        xs_rect.push(&xs[i..i + 7]);
    }
    let mut ys_rect = Vec::new();
    for i in (0..ys.len()).step_by(7) {
        ys_rect.push(&ys[i..i + 7]);
    }

    let mut xx_grad: f32 = 0.0;
    let mut yx_grad: f32 = 0.0;
    for row in &xs_rect {
        for (a, b) in row.iter().zip(row.iter().skip(1)) {
            if b > a {
                xx_grad += 1.;
            }
        }
    }
    for row in &ys_rect {
        for (a, b) in row.iter().zip(row.iter().skip(1)) {
            if b > a {
                yx_grad += 1.;
            }
        }
    }
    let mut xy_grad: f32 = 0.0;
    let mut yy_grad: f32 = 0.0;
    for i in 0..7 {
        for j in 0..14 {
            if xs_rect[j + 1][i] > xs_rect[j][i] {
                xy_grad += 1.;
            }
        }
    }
    for i in 0..7 {
        for j in 0..14 {
            if ys_rect[j + 1][i] > ys_rect[j][i] {
                yy_grad += 1.;
            }
        }
    }
    let grad = ((xx_grad - yx_grad).powf(2.) + (xy_grad - yy_grad).powf(2.));
    let x_intensity: f32 = xs.iter().sum();
    let y_intensity: f32 = ys.iter().sum();
    grad / (1. + (x_intensity - y_intensity).powf(2.))
}
