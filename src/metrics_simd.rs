#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

use std::arch::x86_64::*;

pub fn jaccard_score(xs: &[f32], ys: &[f32]) -> f32 {
    assert_eq!(xs.len(), ys.len());
    let mut intersection_sum;
    let mut union_sum;
    let mut i = 8;

    unsafe {
        let mut xs_ptr = xs.as_ptr();
        let mut ys_ptr = ys.as_ptr();
        let mut xs_data = _mm256_loadu_ps(xs_ptr);
        let mut ys_data = _mm256_loadu_ps(ys_ptr);

        let mut min_sum = _mm256_min_ps(xs_data, ys_data);
        let mut max_sum = _mm256_max_ps(xs_data, ys_data);

        while i + 7 < xs.len() {
            xs_ptr = xs_ptr.add(8);
            ys_ptr = ys_ptr.add(8);

            xs_data = _mm256_loadu_ps(xs_ptr);
            ys_data = _mm256_loadu_ps(ys_ptr);

            let mins = _mm256_min_ps(xs_data, ys_data);
            let maxs = _mm256_max_ps(xs_data, ys_data);

            min_sum = _mm256_add_ps(mins, min_sum);
            max_sum = _mm256_add_ps(maxs, max_sum);

            i += 8;
        }

        let mins_low = _mm256_castps256_ps128(min_sum);
        let mins_high = _mm256_extractf128_ps(min_sum, 1);
        let mins = _mm_add_ps(mins_low, mins_high);
        let mins = _mm_hadd_ps(mins, mins);
        let mins = _mm_hadd_ps(mins, mins);

        let maxs_low = _mm256_castps256_ps128(max_sum);
        let maxs_high = _mm256_extractf128_ps(max_sum, 1);
        let maxs = _mm_add_ps(maxs_low, maxs_high);
        let maxs = _mm_hadd_ps(maxs, maxs);
        let maxs = _mm_hadd_ps(maxs, maxs);

        intersection_sum = _mm_cvtss_f32(mins);
        union_sum = _mm_cvtss_f32(maxs);
    }

    while i < xs.len() {
        intersection_sum += xs[i].min(ys[i]);
        union_sum += xs[i].max(ys[i]);
        i += 1;
    }

    intersection_sum / union_sum
}

pub fn dot_score(xs: &[f32], ys: &[f32]) -> f32 {
    assert_eq!(xs.len(), ys.len());
    let mut total;
    let mut i = 8;

    unsafe {
        let mut xs_ptr = xs.as_ptr();
        let mut ys_ptr = ys.as_ptr();
        let mut xs_data = _mm256_loadu_ps(xs_ptr);
        let mut ys_data = _mm256_loadu_ps(ys_ptr);

        let mut sums = _mm256_mul_ps(xs_data, ys_data);

        while i + 7 < xs.len() {
            xs_ptr = xs_ptr.add(8);
            ys_ptr = ys_ptr.add(8);

            xs_data = _mm256_loadu_ps(xs_ptr);
            ys_data = _mm256_loadu_ps(ys_ptr);

            let mul = _mm256_mul_ps(xs_data, ys_data);

            sums = _mm256_add_ps(sums, mul);

            i += 8;
        }

        let sums_low = _mm256_castps256_ps128(sums);
        let sums_high = _mm256_extractf128_ps(sums, 1);
        let sums = _mm_add_ps(sums_low, sums_high);
        let sums = _mm_hadd_ps(sums, sums);
        let sums = _mm_hadd_ps(sums, sums);

        total = _mm_cvtss_f32(sums);
    }

    while i < xs.len() {
        total += xs[i] * ys[i];
        i += 1;
    }

    total
}
