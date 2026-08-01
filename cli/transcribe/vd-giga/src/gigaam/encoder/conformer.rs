//! Conformer encoder (Candle) matching GigaAM `encoder.py`.

use candle_core::{Device, Result as CandleResult, Tensor, D};
use candle_nn::{
    conv1d, layer_norm, linear, Conv1d, Conv1dConfig, LayerNorm, Linear, Module, VarBuilder,
};

use super::rope::{apply_rope_on_heads, rope_cos_sin_tensors};

#[derive(Debug, Clone)]
pub struct ConformerConfig {
    pub feat_in: usize,
    pub n_layers: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub ff_expansion: usize,
    pub subs_kernel_size: usize,
    pub subsampling_factor: usize,
    pub conv_kernel_size: usize,
    pub pos_emb_max_len: usize,
}

impl ConformerConfig {
    pub fn from_card(c: &crate::gigaam::weights::ModelCard) -> Self {
        Self {
            feat_in: c.encoder.feat_in,
            n_layers: c.encoder.n_layers,
            d_model: c.encoder.d_model,
            n_heads: c.encoder.n_heads,
            ff_expansion: c.encoder.ff_expansion_factor,
            subs_kernel_size: c.encoder.subs_kernel_size,
            subsampling_factor: c.encoder.subsampling_factor,
            conv_kernel_size: c.encoder.conv_kernel_size,
            pos_emb_max_len: c.encoder.pos_emb_max_len,
        }
    }

    pub fn head_dim(&self) -> usize {
        self.d_model / self.n_heads
    }
}

struct FeedForward {
    linear1: Linear,
    linear2: Linear,
}

impl FeedForward {
    fn load(vb: VarBuilder, d_model: usize, d_ff: usize) -> CandleResult<Self> {
        Ok(Self {
            linear1: linear(d_model, d_ff, vb.pp("linear1"))?,
            linear2: linear(d_ff, d_model, vb.pp("linear2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // SiLU
        let x = self.linear1.forward(x)?;
        let x = candle_nn::ops::silu(&x)?;
        self.linear2.forward(&x)
    }
}

struct ConformerConv {
    pointwise1: Conv1d,
    depthwise: Conv1d,
    norm: LayerNorm,
    pointwise2: Conv1d,
}

impl ConformerConv {
    fn load(vb: VarBuilder, d_model: usize, kernel: usize) -> CandleResult<Self> {
        let pw_cfg = Conv1dConfig {
            padding: 0,
            stride: 1,
            dilation: 1,
            groups: 1,
            cudnn_fwd_algo: None,
        };
        let dw_cfg = Conv1dConfig {
            padding: (kernel - 1) / 2,
            stride: 1,
            dilation: 1,
            groups: d_model,
            cudnn_fwd_algo: None,
        };
        Ok(Self {
            pointwise1: conv1d(d_model, d_model * 2, 1, pw_cfg, vb.pp("pointwise_conv1"))?,
            depthwise: conv1d(d_model, d_model, kernel, dw_cfg, vb.pp("depthwise_conv"))?,
            // Named batch_norm in state_dict even when LayerNorm.
            norm: layer_norm(d_model, 1e-5, vb.pp("batch_norm"))?,
            pointwise2: conv1d(d_model, d_model, 1, pw_cfg, vb.pp("pointwise_conv2"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // x: [B, T, D] -> [B, D, T]
        let x = x.transpose(1, 2)?;
        let x = self.pointwise1.forward(&x)?;
        let x = glu_last_half(&x)?;
        let x = self.depthwise.forward(&x)?;
        // LayerNorm over channel dim: transpose to [B,T,D], norm, back.
        let x = x.transpose(1, 2)?;
        let x = self.norm.forward(&x)?;
        let x = x.transpose(1, 2)?;
        let x = candle_nn::ops::silu(&x)?;
        let x = self.pointwise2.forward(&x)?;
        x.transpose(1, 2)
    }
}

fn glu_last_half(x: &Tensor) -> CandleResult<Tensor> {
    // GLU on dim=1 (channels): split in half, a * sigmoid(b)
    let c = x.dim(1)?;
    let half = c / 2;
    let a = x.narrow(1, 0, half)?;
    let b = x.narrow(1, half, half)?;
    let b = candle_nn::ops::sigmoid(&b)?;
    a * b
}

struct RotaryMha {
    n_heads: usize,
    head_dim: usize,
    linear_q: Linear,
    linear_k: Linear,
    linear_v: Linear,
    linear_out: Linear,
}

impl RotaryMha {
    fn load(vb: VarBuilder, d_model: usize, n_heads: usize) -> CandleResult<Self> {
        Ok(Self {
            n_heads,
            head_dim: d_model / n_heads,
            linear_q: linear(d_model, d_model, vb.pp("linear_q"))?,
            linear_k: linear(d_model, d_model, vb.pp("linear_k"))?,
            linear_v: linear(d_model, d_model, vb.pp("linear_v"))?,
            linear_out: linear(d_model, d_model, vb.pp("linear_out"))?,
        })
    }

    /// GigaAM order: reshape to heads → RoPE on q/k → project Q/K/V → attention.
    fn forward(&self, x: &Tensor, cos: &Tensor, sin: &Tensor) -> CandleResult<Tensor> {
        let (b, t, _d) = x.dims3()?;
        // View as [B, T, H, Dh], apply RoPE to copies used as pre-proj q/k.
        let x_heads = x.reshape((b, t, self.n_heads, self.head_dim))?;
        let q_rope = apply_rope_on_heads(&x_heads, cos, sin)?;
        let k_rope = apply_rope_on_heads(&x_heads, cos, sin)?;
        let q_in = q_rope.reshape((b, t, self.n_heads * self.head_dim))?;
        let k_in = k_rope.reshape((b, t, self.n_heads * self.head_dim))?;

        let q = self.linear_q.forward(&q_in)?;
        let k = self.linear_k.forward(&k_in)?;
        let v = self.linear_v.forward(x)?;

        let q = q
            .reshape((b, t, self.n_heads, self.head_dim))?
            .transpose(1, 2)?; // [B,H,T,Dh]
        let k = k
            .reshape((b, t, self.n_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = v
            .reshape((b, t, self.n_heads, self.head_dim))?
            .transpose(1, 2)?;

        let scale = (self.head_dim as f64).sqrt();
        let att = q.matmul(&k.transpose(D::Minus1, D::Minus2)?)?;
        let att = (att / scale)?;
        let att = candle_nn::ops::softmax_last_dim(&att)?;
        let out = att.matmul(&v)?; // [B,H,T,Dh]
        let out = out
            .transpose(1, 2)?
            .reshape((b, t, self.n_heads * self.head_dim))?;
        self.linear_out.forward(&out)
    }
}

struct ConformerLayer {
    norm_ff1: LayerNorm,
    ff1: FeedForward,
    norm_self_att: LayerNorm,
    self_attn: RotaryMha,
    norm_conv: LayerNorm,
    conv: ConformerConv,
    norm_ff2: LayerNorm,
    ff2: FeedForward,
    norm_out: LayerNorm,
}

impl ConformerLayer {
    fn load(vb: VarBuilder, cfg: &ConformerConfig) -> CandleResult<Self> {
        let d_ff = cfg.d_model * cfg.ff_expansion;
        Ok(Self {
            norm_ff1: layer_norm(cfg.d_model, 1e-5, vb.pp("norm_feed_forward1"))?,
            ff1: FeedForward::load(vb.pp("feed_forward1"), cfg.d_model, d_ff)?,
            norm_self_att: layer_norm(cfg.d_model, 1e-5, vb.pp("norm_self_att"))?,
            self_attn: RotaryMha::load(vb.pp("self_attn"), cfg.d_model, cfg.n_heads)?,
            norm_conv: layer_norm(cfg.d_model, 1e-5, vb.pp("norm_conv"))?,
            conv: ConformerConv::load(vb.pp("conv"), cfg.d_model, cfg.conv_kernel_size)?,
            norm_ff2: layer_norm(cfg.d_model, 1e-5, vb.pp("norm_feed_forward2"))?,
            ff2: FeedForward::load(vb.pp("feed_forward2"), cfg.d_model, d_ff)?,
            norm_out: layer_norm(cfg.d_model, 1e-5, vb.pp("norm_out"))?,
        })
    }

    fn forward(&self, x: &Tensor, cos: &Tensor, sin: &Tensor) -> CandleResult<Tensor> {
        let residual = x;
        let x = self.norm_ff1.forward(x)?;
        let x = self.ff1.forward(&x)?;
        let residual = (residual + (x * 0.5)?)?;

        let x = self.norm_self_att.forward(&residual)?;
        let x = self.self_attn.forward(&x, cos, sin)?;
        let residual = (&residual + x)?;

        let x = self.norm_conv.forward(&residual)?;
        let x = self.conv.forward(&x)?;
        let residual = (&residual + x)?;

        let x = self.norm_ff2.forward(&residual)?;
        let x = self.ff2.forward(&x)?;
        let residual = (&residual + (x * 0.5)?)?;

        self.norm_out.forward(&residual)
    }
}

struct Subsampling {
    convs: Vec<Conv1d>,
    kernel: usize,
    stride: usize,
    stages: usize,
}

impl Subsampling {
    fn load(vb: VarBuilder, cfg: &ConformerConfig) -> CandleResult<Self> {
        let stages = (cfg.subsampling_factor as f32).log2() as usize;
        let padding = (cfg.subs_kernel_size - 1) / 2;
        let conv_cfg = Conv1dConfig {
            padding,
            stride: 2,
            dilation: 1,
            groups: 1,
            cudnn_fwd_algo: None,
        };
        let mut convs = Vec::with_capacity(stages);
        let mut in_ch = cfg.feat_in;
        // state_dict indices: 0, 2 (ReLU at 1, 3)
        for i in 0..stages {
            let key = format!("{}", i * 2);
            convs.push(conv1d(
                in_ch,
                cfg.d_model,
                cfg.subs_kernel_size,
                conv_cfg,
                vb.pp("conv").pp(key),
            )?);
            in_ch = cfg.d_model;
        }
        Ok(Self {
            convs,
            kernel: cfg.subs_kernel_size,
            stride: 2,
            stages,
        })
    }

    fn out_len(&self, len: usize) -> usize {
        let add_pad = 2 * ((self.kernel - 1) / 2) as i64 - self.kernel as i64;
        let mut l = len as i64;
        for _ in 0..self.stages {
            l = (l + add_pad) / self.stride as i64 + 1;
        }
        l.max(0) as usize
    }

    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // x: [B, T, F] -> [B, F, T]
        let mut x = x.transpose(1, 2)?;
        for conv in &self.convs {
            x = conv.forward(&x)?;
            x = x.relu()?;
        }
        // [B, D, T] -> [B, T, D]
        x.transpose(1, 2)
    }
}

pub struct ConformerEncoder {
    pub config: ConformerConfig,
    pre_encode: Subsampling,
    layers: Vec<ConformerLayer>,
    device: Device,
}

impl ConformerEncoder {
    pub fn load(vb: VarBuilder, cfg: ConformerConfig, device: &Device) -> CandleResult<Self> {
        let mut layers = Vec::with_capacity(cfg.n_layers);
        for i in 0..cfg.n_layers {
            layers.push(ConformerLayer::load(
                vb.pp("layers").pp(format!("{i}")),
                &cfg,
            )?);
        }
        Ok(Self {
            pre_encode: Subsampling::load(vb.pp("pre_encode"), &cfg)?,
            layers,
            config: cfg,
            device: device.clone(),
        })
    }

    /// `features`: [B, T, feat_in] log-mel. Returns encoded [B, D, T'] and length.
    pub fn forward(&self, features: &Tensor, input_len: usize) -> CandleResult<(Tensor, usize)> {
        let x = self.pre_encode.forward(features)?;
        let t = x.dim(1)?;
        let enc_len = self.pre_encode.out_len(input_len).min(t);

        let (cos, sin) = rope_cos_sin_tensors(
            self.config.pos_emb_max_len.max(t),
            self.config.head_dim(),
            10_000.0,
            &self.device,
        )?;

        let mut x = x;
        for layer in &self.layers {
            x = layer.forward(&x, &cos, &sin)?;
        }
        // [B, T, D] -> [B, D, T] for CTC head
        Ok((x.transpose(1, 2)?, enc_len))
    }
}

pub struct CtcHead {
    conv: Conv1d,
}

impl CtcHead {
    pub fn load(vb: VarBuilder, feat_in: usize, num_classes: usize) -> CandleResult<Self> {
        let cfg = Conv1dConfig {
            padding: 0,
            stride: 1,
            dilation: 1,
            groups: 1,
            cudnn_fwd_algo: None,
        };
        Ok(Self {
            conv: conv1d(
                feat_in,
                num_classes,
                1,
                cfg,
                vb.pp("decoder_layers").pp("0"),
            )?,
        })
    }

    /// `encoded`: [B, D, T] → log_probs [B, T, C]
    pub fn forward(&self, encoded: &Tensor) -> CandleResult<Tensor> {
        let x = self.conv.forward(encoded)?; // [B, C, T]
        let x = x.transpose(1, 2)?; // [B, T, C]
        candle_nn::ops::log_softmax(&x, D::Minus1)
    }
}
