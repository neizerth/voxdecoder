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
