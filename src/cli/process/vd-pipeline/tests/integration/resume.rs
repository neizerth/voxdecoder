//! Resume test: same input file → same cache_key → reuse cached output (P1-H).
//!
//! Does not spawn real ASR subprocesses (no binaries available in test env — this is why
//! the first P1-H attempt was abandoned, per ADR 0017's Implementation status notes). Instead
//! proves the plumbing that makes resume possible: two independent `resolve_job()` calls over
//! the same content compute the *same on-disk cache directory*, so a file a first run wrote
//! there is genuinely visible to a second run — not just "the key strings match" but "the
//! directory a real step would look in is the literal same directory". The reuse-vs-regenerate
//! decision itself (`reuse_existing()`) is unit-tested directly in
//! `vd_pipeline::exec::subprocess::tests`.

#[cfg(test)]
mod tests {
    use std::fs;
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

    #[test]
    fn second_run_sees_first_runs_cache_directory() {
        // Simulates a real resume: run 1 completes a step and writes its output under
        // its resolved cache dir; run 2 (fresh Job, same input content) must resolve to
        // the exact same directory on disk, so the file run 1 wrote is genuinely visible
        // to whatever step run 2 would execute next — not just equal key strings.
        let temp = TempDir::new().unwrap();
        let audio_path = temp.path().join("test.wav");
        fs::write(&audio_path, b"identical content for both runs").unwrap();

        let build_resolved = || {
            let args = DefaultJobArgs {
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
            resolve_job(default_job(&args)).expect("job should resolve")
        };

        // Run 1: resolve, then simulate a completed preprocess step writing into its
        // resolved cache dir.
        let resolved1 = build_resolved();
        let cache_dir1 = vd_artifact::job_cache_dir(&resolved1.cache_key);
        fs::create_dir_all(&cache_dir1).unwrap();
        let marker = cache_dir1.join("preprocess.done");
        fs::write(&marker, b"completed by run 1").unwrap();

        // Run 2: independent Job/resolve_job call over the same input content.
        let resolved2 = build_resolved();
        let cache_dir2 = vd_artifact::job_cache_dir(&resolved2.cache_key);

        assert_eq!(
            cache_dir1, cache_dir2,
            "both runs must resolve to the identical on-disk cache directory"
        );

        // The literal file run 1 wrote must be readable via run 2's independently
        // resolved path — this is what "resume" means at the filesystem level.
        let seen_from_run2 = cache_dir2.join("preprocess.done");
        assert!(
            seen_from_run2.exists(),
            "run 2 must see run 1's completed step output at its own resolved cache path"
        );
        assert_eq!(
            fs::read_to_string(&seen_from_run2).unwrap(),
            "completed by run 1"
        );
    }
}
