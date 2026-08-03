//! Fixed-window chunking so single-pass Conformer stays within GigaAM's ~25s training limit.
//!
//! Long audio otherwise allocates O(T²) attention buffers on Metal and hits
//! `Failed to create metal resource: Buffer`.

/// Max samples per forward pass (20s @ 16 kHz). Under the official 25s limit for headroom.
pub const MAX_CHUNK_SAMPLES: usize = 20 * 16_000;

/// Inclusive-exclusive windows covering `n_samples` without overlap.
pub fn chunk_ranges(n_samples: usize, max_chunk: usize) -> Vec<(usize, usize)> {
    if n_samples == 0 {
        return Vec::new();
    }
    let max_chunk = max_chunk.max(1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < n_samples {
        let end = (start + max_chunk).min(n_samples);
        out.push((start, end));
        start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_one_chunk() {
        assert_eq!(chunk_ranges(1000, MAX_CHUNK_SAMPLES), vec![(0, 1000)]);
    }

    #[test]
    fn splits_long_audio() {
        let n = MAX_CHUNK_SAMPLES * 2 + 100;
        let ranges = chunk_ranges(n, MAX_CHUNK_SAMPLES);
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0], (0, MAX_CHUNK_SAMPLES));
        assert_eq!(ranges[1], (MAX_CHUNK_SAMPLES, MAX_CHUNK_SAMPLES * 2));
        assert_eq!(ranges[2], (MAX_CHUNK_SAMPLES * 2, n));
    }

    #[test]
    fn empty() {
        assert!(chunk_ranges(0, MAX_CHUNK_SAMPLES).is_empty());
    }
}
