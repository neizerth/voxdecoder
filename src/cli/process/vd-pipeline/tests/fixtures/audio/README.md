# Audio fixtures for vd-pipeline e2e

| File | Source |
|------|--------|
| `02-03-strannoye-hobby.mp3` | [dialogue](https://www.russianforfree.com/resources/audio_dialogues/02-03-strannoye-hobby.mp3) |
| `02-03-strannoye-hobby.expected.txt` | Reference transcript (stress marks removed) |
| `text-beginner-russian-sauna.mp3` | [text](https://www.russianforfree.com/resources/audio_texts/text-beginner-russian-sauna.mp3) |
| `text-beginner-russian-sauna.expected.txt` | Reference transcript (stress marks removed) |

Used by gated ASR e2e (`VD_PIPELINE_E2E_FULL=1`). Needs converted CTC weights
(`vd-gigaam/models/v3_e2e_ctc`) — RNNT is not runnable yet. E2E adds
`device: metal` only when the sibling `vd-gigaam` lists Metal in `--help`.

## Speed experiment (`VD_PIPELINE_E2E_SPEED=1`)

Compares preprocess `speed` factors on these clips (real `ffmpeg` + `vd-preprocess`
+ `vd-gigaam`). Asserts sped-up runs are faster than 1× and word coverage does
not collapse vs the 1× baseline.

Bands (`VD_PIPELINE_E2E_SPEED_BAND`):

| band | factors |
|------|---------|
| `low` | `1.0 / 1.25 / 1.5 / 1.75 / 2.0` |
| `high` | `1.0` + `2.0 / 2.25 / 2.5 / 2.75 / 3.0 / 3.5 / 4.0` |
| `all` (default) | union of low + high |

Factors above 2× use chained `atempo` in `vd-preprocess` (ffmpeg limit per stage).
Hard accuracy asserts apply through 3×; 3.5–4× are measured for speed with a soft
coverage floor (clips may cliff — see test notes).

```bash
# optional: only one clip — hobby | sauna | all (default)
export VD_PIPELINE_E2E_SPEED_CLIP=sauna
# optional: low | high | all (default)
export VD_PIPELINE_E2E_SPEED_BAND=high
VD_PIPELINE_E2E_SPEED=1 cargo test --release -p vd-pipeline --test e2e \
  preprocess_speed_faster_than_1x -- --ignored --nocapture
```

## TimeMap remap (`VD_PIPELINE_E2E_TIMEMAP=1`)

Runs the same clip at 1× and with preprocess `speed: 2`. Asserts that remapped
`transcript.segments.json` utterance end matches 1× within a small tolerance
(ADR §5–6).

```bash
VD_PIPELINE_E2E_TIMEMAP=1 cargo test --release -p vd-pipeline --test e2e \
  preprocess_speed_2x_timemap_matches_1x_segments -- --ignored --nocapture
```
