//! Rotary positional embedding helpers (GigaAM `apply_rotary_pos_emb` / `rtt_half`).
//!
//! Applied **before** Q/K projections in GigaAM's rotary MHA.

use candle_core::{Device, Result as CandleResult, Tensor, D};

/// Rotate half: `[-x2, x1]` on the last dimension (even width).
pub fn rotate_half(x: &[f32]) -> Vec<f32> {
    assert_eq!(x.len() % 2, 0, "RoPE last dim must be even");
    let half = x.len() / 2;
    let mut out = Vec::with_capacity(x.len());
    out.extend(x[half..].iter().map(|v| -v));
    out.extend_from_slice(&x[..half]);
    out
}

/// `out = x * cos + rotate_half(x) * sin` (elementwise), matching GigaAM utils.
pub fn apply_rope_inplace(x: &mut [f32], cos: &[f32], sin: &[f32]) {
    assert_eq!(x.len(), cos.len());
    assert_eq!(x.len(), sin.len());
    let rotated = rotate_half(x);
    for i in 0..x.len() {
        x[i] = x[i] * cos[i] + rotated[i] * sin[i];
    }
}

/// Build cos/sin tables for rotary dim `head_dim` over `length` positions (base 10_000).
pub fn rope_cos_sin(length: usize, head_dim: usize, base: f32) -> (Vec<f32>, Vec<f32>) {
    assert_eq!(head_dim % 2, 0);
    let half = head_dim / 2;
    let mut cos = vec![0.0f32; length * head_dim];
    let mut sin = vec![0.0f32; length * head_dim];
    for t in 0..length {
        for i in 0..half {
            let freq = 1.0 / base.powf((2 * i) as f32 / head_dim as f32);
            let angle = t as f32 * freq;
            let c = angle.cos();
            let s = angle.sin();
            cos[t * head_dim + i] = c;
            cos[t * head_dim + half + i] = c;
            sin[t * head_dim + i] = s;
            sin[t * head_dim + half + i] = s;
        }
    }
    (cos, sin)
}

/// Cos/sin as `[T, 1, 1, Dh]` for broadcast over batch/heads.
pub fn rope_cos_sin_tensors(
    length: usize,
    head_dim: usize,
    base: f32,
    device: &Device,
) -> CandleResult<(Tensor, Tensor)> {
    let (cos, sin) = rope_cos_sin(length, head_dim, base);
    let cos = Tensor::from_vec(cos, (length, 1, 1, head_dim), device)?;
    let sin = Tensor::from_vec(sin, (length, 1, 1, head_dim), device)?;
    Ok((cos, sin))
}

/// Apply RoPE to `x` with shape `[B, T, H, Dh]`.
/// `cos` / `sin` must be `[T_full, 1, 1, Dh]`.
pub fn apply_rope_on_heads(x: &Tensor, cos: &Tensor, sin: &Tensor) -> CandleResult<Tensor> {
    let (_b, t, _h, dh) = x.dims4()?;
    let cos = cos.narrow(0, 0, t)?.reshape((1, t, 1, dh))?;
    let sin = sin.narrow(0, 0, t)?.reshape((1, t, 1, dh))?;
    let rotated = rotate_half_tensor(x)?;
    let a = x.broadcast_mul(&cos)?;
    let b = rotated.broadcast_mul(&sin)?;
    a + b
}

fn rotate_half_tensor(x: &Tensor) -> CandleResult<Tensor> {
    let dh = x.dim(D::Minus1)?;
    let half = dh / 2;
    let x1 = x.narrow(D::Minus1, 0, half)?;
    let x2 = x.narrow(D::Minus1, half, half)?;
    Tensor::cat(&[&(&x2 * (-1f64))?, &x1], D::Minus1)
}
