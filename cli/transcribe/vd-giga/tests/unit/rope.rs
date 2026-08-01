//! RoPE helper unit tests.

use vd_giga::gigaam::encoder::rope::{apply_rope_inplace, rope_cos_sin, rotate_half};

#[test]
fn rotate_half_basic() {
    let x = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(rotate_half(&x), vec![-3.0, -4.0, 1.0, 2.0]);
}

#[test]
fn rope_is_norm_preserving_approx() {
    let mut x = vec![0.5f32, -0.25, 0.75, 0.1];
    let (cos, sin) = rope_cos_sin(1, 4, 10_000.0);
    let before: f32 = x.iter().map(|v| v * v).sum::<f32>().sqrt();
    apply_rope_inplace(&mut x, &cos[..4], &sin[..4]);
    let after: f32 = x.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!((before - after).abs() < 1e-5);
}
