//! Detect Metal GPU allocation failures so callers can retry on CPU.

/// True when Candle/Metal failed to allocate a buffer or similar GPU resource.
///
/// Matches the common Candle surface: `Metal error Failed to create metal resource: Buffer`.
pub fn is_metal_resource_error(message: &str) -> bool {
    let s = message.to_ascii_lowercase();
    if s.contains("failed to create metal resource") {
        return true;
    }
    if s.contains("metal error") && s.contains("buffer") {
        return true;
    }
    s.contains("mtlbuffer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_candle_buffer_message() {
        assert!(is_metal_resource_error(
            "transcription failed: Metal error Failed to create metal resource: Buffer"
        ));
    }

    #[test]
    fn ignores_unrelated() {
        assert!(!is_metal_resource_error("transcription failed: empty audio"));
        assert!(!is_metal_resource_error("model load failed: file not found"));
    }
}
