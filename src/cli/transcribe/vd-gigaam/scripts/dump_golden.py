#!/usr/bin/env python3
"""Dump GigaAM reference log-mel tensors for fixtures.

Requires torch + torchaudio (see requirements.txt).
"""

from __future__ import annotations

import argparse
import json
import wave
from pathlib import Path

import numpy as np
import torch
import torchaudio


def load_wav_mono(path: Path) -> tuple[torch.Tensor, int]:
    with wave.open(str(path), "rb") as w:
        sr = w.getframerate()
        nch = w.getnchannels()
        sw = w.getsampwidth()
        raw = w.readframes(w.getnframes())
    if sw != 2:
        raise SystemExit(f"unsupported sample width {sw}")
    x = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
    if nch > 1:
        x = x.reshape(-1, nch).mean(axis=1)
    return torch.from_numpy(x.copy()), sr


def log_mel(wav: torch.Tensor, sample_rate: int = 16000) -> torch.Tensor:
    """Match GigaAM ASR YAML: 64 mels, n_fft=win=320, hop=160, center=False, HTK, ln."""
    mel = torchaudio.transforms.MelSpectrogram(
        sample_rate=sample_rate,
        n_mels=64,
        n_fft=320,
        win_length=320,
        hop_length=160,
        center=False,
        mel_scale="htk",
        power=2.0,
        norm=None,
    )
    spec = mel(wav)
    return torch.log(spec.clamp(1e-9, 1e9))


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("wav", type=Path)
    p.add_argument("-o", "--out", type=Path, required=True, help="Output .npy or .json")
    args = p.parse_args()

    wav, sr = load_wav_mono(args.wav)
    if sr != 16000:
        wav = torchaudio.functional.resample(wav, sr, 16000)

    feats = log_mel(wav)  # [n_mels, time]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    if args.out.suffix == ".json":
        payload = {
            "shape": list(feats.shape),
            "layout": "n_mels,time",
            "data": feats.flatten().tolist(),
        }
        args.out.write_text(json.dumps(payload))
    else:
        np.save(args.out, feats.numpy())
    print(f"wrote {args.out} shape={tuple(feats.shape)}")


if __name__ == "__main__":
    main()
