//! CTC greedy collapse + blank removal (GigaAM `CTCGreedyDecoding` core).

/// Collapse CTC argmax path: drop `blank_id`, then drop consecutive duplicates.
pub fn greedy_collapse(labels: &[u32], blank_id: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity(labels.len());
    let mut prev: Option<u32> = None;
    for &lab in labels {
        if lab == blank_id {
            prev = Some(lab);
            continue;
        }
        if prev == Some(lab) {
            continue;
        }
        out.push(lab);
        prev = Some(lab);
    }
    out
}

/// Same as [`greedy_collapse`], also returning frame indices of kept tokens.
pub fn greedy_collapse_with_frames(labels: &[u32], blank_id: u32) -> (Vec<u32>, Vec<usize>) {
    let mut tokens = Vec::new();
    let mut frames = Vec::new();
    let mut prev: Option<u32> = None;
    for (t, &lab) in labels.iter().enumerate() {
        if lab == blank_id {
            prev = Some(lab);
            continue;
        }
        if prev == Some(lab) {
            continue;
        }
        tokens.push(lab);
        frames.push(t);
        prev = Some(lab);
    }
    (tokens, frames)
}
