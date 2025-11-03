#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

use std::arch::x86_64::*;


// Reduce a vector of 8 single precision accumulators to
// a single f32 sum. Normally, you wouldn't care much about
// the timing of this since it the "hot" loop typically operates
// on vectors, but we're expecting this to be called in a hot
// loop as well, so the sum time is critical.
#[inline]
unsafe fn _mm256_reduce_sum(v: __m256) -> f32 {
    let low = _mm256_castps256_ps128(v);
    let high = _mm256_extractf128_ps(v, 1);
    let mut v = _mm_add_ps(low, high);
    v = _mm_hadd_ps(v, v);
    v = _mm_hadd_ps(v, v);
    _mm_cvtss_f32(v)
}

pub fn jaccard_score(xs: &[f32], ys: &[f32]) -> f32 {
    assert_eq!(xs.len(), ys.len());
    let mut intersection_sum;
    let mut union_sum;
    let mut i = 8; // loop peeling (see below)

    unsafe {
        let xs_ptr = xs.as_ptr();
        let ys_ptr = ys.as_ptr();
        
        // "loop peeling" - it's better to start with the
        // first sum than to initialize things to 0.
        let mut xs_data = _mm256_loadu_ps(xs_ptr);
        let mut ys_data = _mm256_loadu_ps(ys_ptr);
        let mut min_sum = _mm256_min_ps(xs_data, ys_data);
        let mut max_sum = _mm256_max_ps(xs_data, ys_data);

        while i + 8 <= xs.len() {
            xs_data = _mm256_loadu_ps(xs_ptr.add(i));
            ys_data = _mm256_loadu_ps(ys_ptr.add(i));

            let mins = _mm256_min_ps(xs_data, ys_data);
            let maxs = _mm256_max_ps(xs_data, ys_data);

            min_sum = _mm256_add_ps(mins, min_sum);
            max_sum = _mm256_add_ps(maxs, max_sum);

            i += 8;
        }

        intersection_sum = _mm256_reduce_sum(min_sum);
        union_sum = _mm256_reduce_sum(max_sum);
    }

    // Deal with any extras when the vectors aren't multiples of 8 in
    // length.
    while i < xs.len() {
        intersection_sum += xs[i].min(ys[i]);
        union_sum += xs[i].max(ys[i]);
        i += 1;
    }

    intersection_sum / union_sum
}

pub fn dot_score(xs: &[f32], ys: &[f32]) -> f32 {
    assert_eq!(xs.len(), ys.len());
    let mut i = 8; // loop peeling (see below)

    let mut total = unsafe {
        let xs_ptr = xs.as_ptr();
        let ys_ptr = ys.as_ptr();

        // "loop peeling" - it's better to start with the
        // first sum than to initialize things to 0.
        let mut xs_data = _mm256_loadu_ps(xs_ptr);
        let mut ys_data = _mm256_loadu_ps(ys_ptr);
        let mut sums = _mm256_mul_ps(xs_data, ys_data);

        while i + 8 <= xs.len() {
            xs_data = _mm256_loadu_ps(xs_ptr.add(i));
            ys_data = _mm256_loadu_ps(ys_ptr.add(i));
            sums = _mm256_fmadd_ps(xs_data, ys_data, sums);
            i += 8;
        }

        _mm256_reduce_sum(sums)
    };

    // Deal with any extras when the vectors aren't multiples of 8 in
    // length.
    while i < xs.len() {
        total += xs[i] * ys[i];
        i += 1;
    }

    total
}

#[cfg(test)]
mod tests {
    use std::time;

    use super::{dot_score as simd_dot_score, jaccard_score as simd_jaccard_score};
    use crate::metrics::{dot_score, jaccard_score};

    #[test]
    fn test_dot() {
        let a = vec![1., 2., 3., 4., 5., -1., 2., 0., 3., 1., 1.];
        let b = vec![-1., 3., 4., 5., -1., 3., 4., 3., 5., 2., 1.];
        let p = simd_dot_score(&a, &b);
        assert_eq!(p, 55.);
    }

    #[test]
    fn test_jaccard() {
        let a = vec![1., 2., 3., 4., 5., -1., 2., 0., 3., 1., 1.];
        let b = vec![-1., 3., 4., 5., -1., 3., 4., 3., 5., 2., 1.];
        let p = simd_jaccard_score(&a, &b);
        assert_eq!(p, 13. / 36.);
    }

    #[test]
    fn bench_dot() {
        // we expect a ~7x speedup from using avx here when compiling with RUSTFLAGS='-C target-cpu=native'
        let iterations = 10000000;
        let size = 7 * 13; // size of bitocra characters

        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..size {
            a.push(i as f32);
            b.push(i as f32)
        }

        let p = dot_score(&a, &b);

        let t0 = time::Instant::now();
        let mut total1 = 0.;
        for _ in 0..iterations {
            total1 += dot_score(&a, &b);
        }
        println!("reg:  {:?}", t0.elapsed());

        println!("expected sum: {}", p);
        let t0 = time::Instant::now();
        let mut total2 = 0.;
        for _ in 0..iterations {
            total2 += simd_dot_score(&a, &b);
        }
        assert_eq!(total1, total2);
        println!("simd: {:?}", t0.elapsed());
    }

    #[test]
    fn bench_jaccard() {
        // we expect a ~10x speedup from using avx here when compiling with RUSTFLAGS='-C target-cpu=native'
        let iterations = 10000000;
        let size = 7 * 13; // size of bitocra characters

        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..size {
            a.push(i as f32);
            b.push(i as f32)
        }

        let p = jaccard_score(&a, &b);

        let t0 = time::Instant::now();
        let mut total1 = 0.;
        for _ in 0..iterations {
            total1 += jaccard_score(&a, &b);
        }
        println!("reg:  {:?}", t0.elapsed());

        println!("expected sum: {}", p);
        let t0 = time::Instant::now();
        let mut total2 = 0.;
        for _ in 0..iterations {
            total2 += simd_jaccard_score(&a, &b);
        }
        assert_eq!(total1, total2);
        println!("simd: {:?}", t0.elapsed());
    }
}
