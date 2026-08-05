//! Resume test: same input file → same cache_key → reuse cached output (P1-H).

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use vd_pipeline::job::{default_job, resolve_job, DefaultJobArgs, TranscribeEngine};

    #[test]
    fn same_input_generates_same_cache_key() {
        // Create a temp audio file with deterministic content
        let temp = TempDir::new().unwrap();
        let audio_path = temp.path().join("test.wav");
        let test_content = b"RIFF\x00\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x02\x00D\xac\x00\x00\x10\xb1\x02\x00\x04\x00\x10\x00data\x00\x00\x00\x00";
        fs::write(&audio_path, test_content).unwrap();

        // Build two Jobs from the same input
        let args1 = DefaultJobArgs {
            audio: audio_path.clone(),
            engine: TranscribeEngine::Gigaam,
            model: None,
            device: None,
            flash: false,
            speed: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        };

        let args2 = DefaultJobArgs {
            audio: audio_path.clone(),
            engine: TranscribeEngine::Gigaam,
            model: None,
            device: None,
            flash: false,
            speed: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        };

        let job1 = default_job(&args1);
        let job2 = default_job(&args2);

        // Resolve both jobs
        let resolved1 = resolve_job(job1).expect("job1 should resolve");
        let resolved2 = resolve_job(job2).expect("job2 should resolve");

        // Same input file should generate the same cache_key (content hash)
        assert_eq!(
            resolved1.cache_key, resolved2.cache_key,
            "same audio file should produce same cache_key for deduplication"
        );

        // Verify that cache_key is a BLAKE3 hash, not a minted id
        // BLAKE3 hex: exactly 64 chars (256 bits in hex)
        assert_eq!(
            resolved1.cache_key.len(),
            64,
            "cache_key should be BLAKE3 hex hash (64 chars), not minted id"
        );
        assert!(
            resolved1.cache_key.chars().all(|c| c.is_ascii_hexdigit()),
            "cache_key should be valid hex"
        );
    }

    #[test]
    fn different_inputs_generate_different_cache_keys() {
        let temp = TempDir::new().unwrap();

        // Create two different audio files
        let audio1 = temp.path().join("test1.wav");
        let audio2 = temp.path().join("test2.wav");

        fs::write(&audio1, b"RIFF\x00\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x02").unwrap();
        fs::write(&audio2, b"RIFF\x00\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x03").unwrap(); // Different byte

        let args1 = DefaultJobArgs {
            audio: audio1,
            engine: TranscribeEngine::Gigaam,
            model: None,
            device: None,
            flash: false,
            speed: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        };

        let args2 = DefaultJobArgs {
            audio: audio2,
            engine: TranscribeEngine::Gigaam,
            model: None,
            device: None,
            flash: false,
            speed: None,
            docs: None,
            output_dir: None,
            working_dir: None,
            continue_on_error: false,
            overwrite: false,
        };

        let job1 = default_job(&args1);
        let job2 = default_job(&args2);

        let resolved1 = resolve_job(job1).expect("job1 should resolve");
        let resolved2 = resolve_job(job2).expect("job2 should resolve");

        assert_ne!(
            resolved1.cache_key, resolved2.cache_key,
            "different audio files should produce different cache_keys"
        );
    }
}
