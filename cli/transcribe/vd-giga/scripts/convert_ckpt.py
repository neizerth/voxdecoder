#!/usr/bin/env python3
"""Convert a GigaAM ``.ckpt`` to SafeTensors + ``model.json`` for Candle.

Maintainer / CI only — not a runtime dependency of ``vd-giga``.

Example:
  python convert_ckpt.py ~/.cache/gigaam/v3_e2e_ctc.ckpt -o ./models/v3_e2e_ctc
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

import torch
from omegaconf import OmegaConf
from safetensors.torch import save_file


def _to_plain(obj):
    if OmegaConf.is_config(obj):
        return OmegaConf.to_container(obj, resolve=True)
    return obj


def convert(ckpt_path: Path, out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    ckpt = torch.load(ckpt_path, map_location="cpu", weights_only=False)
    cfg = ckpt["cfg"]
    state = ckpt["state_dict"]

    # Drop preprocessor buffers we recompute in Rust (mel is not in weights).
    tensors = {
        k: v.contiguous().cpu()
        for k, v in state.items()
        if isinstance(v, torch.Tensor) and not k.startswith("preprocessor.")
    }
    # FP32 for CPU Candle path.
    tensors = {k: (v.float() if v.is_floating_point() else v) for k, v in tensors.items()}

    st_path = out_dir / "model.safetensors"
    save_file(tensors, str(st_path))

    model_name = str(getattr(cfg, "model_name", ckpt_path.stem))
    decoding = _to_plain(cfg.get("decoding")) or {}
    encoder = _to_plain(cfg.get("encoder")) or {}
    head = _to_plain(cfg.get("head")) or {}
    preprocessor = _to_plain(cfg.get("preprocessor")) or {}

    meta = {
        "model_name": model_name,
        "decoder": "ctc" if "ctc" in model_name else "rnnt",
        "encoder": {
            "feat_in": encoder.get("feat_in", 64),
            "n_layers": encoder.get("n_layers", 16),
            "d_model": encoder.get("d_model", 768),
            "n_heads": encoder.get("n_heads", 16),
            "ff_expansion_factor": encoder.get("ff_expansion_factor", 4),
            "subsampling": encoder.get("subsampling", "conv1d"),
            "subs_kernel_size": encoder.get("subs_kernel_size", 5),
            "subsampling_factor": encoder.get("subsampling_factor", 4),
            "self_attention_model": encoder.get("self_attention_model", "rotary"),
            "conv_kernel_size": encoder.get("conv_kernel_size", 31),
            "conv_norm_type": encoder.get("conv_norm_type", "batch_norm"),
            "pos_emb_max_len": encoder.get("pos_emb_max_len", 5000),
        },
        "head": {
            "feat_in": head.get("feat_in", encoder.get("d_model", 768)),
            "num_classes": head.get("num_classes"),
        },
        "preprocessor": {
            "sample_rate": preprocessor.get("sample_rate", 16000),
            "features": preprocessor.get("features", 64),
            "n_fft": preprocessor.get("n_fft", 320),
            "win_length": preprocessor.get("win_length", 320),
            "hop_length": preprocessor.get("hop_length", 160),
            "center": preprocessor.get("center", False),
        },
        "decoding": {
            "vocabulary": decoding.get("vocabulary"),
            "tokenizer_file": None,
        },
        "tensors": sorted(tensors.keys()),
    }

    # Copy SentencePiece if present next to ckpt or referenced in cfg.
    tok_src = None
    model_path = decoding.get("model_path")
    if model_path and Path(model_path).is_file():
        tok_src = Path(model_path)
    else:
        sibling = ckpt_path.with_name(f"{ckpt_path.stem}_tokenizer.model")
        if sibling.is_file():
            tok_src = sibling
    if tok_src is not None:
        tok_dst = out_dir / "tokenizer.model"
        shutil.copy2(tok_src, tok_dst)
        meta["decoding"]["tokenizer_file"] = "tokenizer.model"
        try:
            import sentencepiece as spm

            sp = spm.SentencePieceProcessor(model_file=str(tok_src))
            meta["decoding"]["pieces"] = [
                sp.id_to_piece(i) for i in range(sp.get_piece_size())
            ]
        except Exception as exc:  # noqa: BLE001
            print(f"warning: could not export SPM pieces: {exc}")

    (out_dir / "model.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n")
    print(f"wrote {st_path} ({len(tensors)} tensors)")
    print(f"wrote {out_dir / 'model.json'}")
    if meta["decoding"]["tokenizer_file"]:
        print(f"wrote {out_dir / 'tokenizer.model'}")
        n = len(meta["decoding"].get("pieces") or [])
        if n:
            print(f"embedded {n} SPM pieces in model.json")


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("ckpt", type=Path, help="Path to .ckpt")
    p.add_argument("-o", "--out", type=Path, required=True, help="Output directory")
    args = p.parse_args()
    convert(args.ckpt.expanduser(), args.out.expanduser())


if __name__ == "__main__":
    main()
