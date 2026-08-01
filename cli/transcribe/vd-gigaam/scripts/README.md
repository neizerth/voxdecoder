//! Maintainer / CI only — not linked into the binary.

## Convert checkpoint → SafeTensors

```bash
cd cli/transcribe/vd-gigaam/scripts
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
python convert_ckpt.py ~/.cache/gigaam/v3_e2e_ctc.ckpt -o ../models/v3_e2e_ctc
```

Writes `model.safetensors`, `model.json` (incl. SPM pieces), and `tokenizer.model`.

`vd-gigaam install MODEL` downloads the `.ckpt` from the GigaAM CDN, verifies MD5, then
runs this script automatically when `scripts/convert_ckpt.py` (or `VD_GIGAAM_CONVERT_SCRIPT`)
and a Python with deps (or `scripts/.venv`) are available.

## Golden mel dump

```bash
python dump_golden.py path/to.wav -o ../fixtures/golden/example_mel.npy
```

## Run converted CTC model

```bash
cargo run -p vd-gigaam -- run -i sample.wav -m v3_e2e_ctc \
  --download-root cli/transcribe/vd-gigaam/models --device cpu
```

Runtime path stays Rust-only (see README).
