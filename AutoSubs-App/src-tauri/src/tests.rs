use crate::transcription_api::{FrontendTranscribeOptions, transcribe_audio};
use std::fs;
use tauri::test::{mock_builder, mock_context, noop_assets};

#[cfg(test)]
mod tests {
    use super::*;

    // run with cargo test transcribe_audio_smoke -- --nocapture
    #[tokio::test(flavor = "multi_thread")]
    async fn transcribe_audio_smoke() {
        let app = mock_builder()
            .plugin(tauri_plugin_shell::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build test app");
        let handle = app.handle().clone();

        // Use a portable test asset path (avoid absolute machine paths)
        let wav = format!("{}/tests/data/test-audio.wav", env!("CARGO_MANIFEST_DIR"));

        let options = FrontendTranscribeOptions {
            audio_path: wav,
            offset: None,
            model: "moonshine-tiny".into(),
            lang: Some("en".into()),
            translate: Some(false),
            target_language: None,
            enable_dtw: Some(false),
            enable_gpu: Some(true),
            enable_diarize: Some(false),
            max_speakers: None,
            density: None,
            max_lines: None,
            custom_max_chars_per_line: None,
            text_case: None,
            remove_punctuation: None,
            censored_words: None,
            custom_prompt: None,
        };

        let res = transcribe_audio(handle, options).await;
        assert!(res.is_ok(), "transcription failed: {:?}", res.err());

        // Save resulting transcript to tests/data for inspection
        if let Ok(transcript) = res {
            let out_path = format!(
                "{}/tests/data/transcript-smoke.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let json =
                serde_json::to_string_pretty(&transcript).expect("failed to serialize transcript");
            fs::write(&out_path, json).expect("failed to write transcript file");
            eprintln!("Saved transcript to {}", out_path);
        }
    }

    // Runs transcription while ensuring VAD model is present; saves a VAD transcript snapshot.
    // run with: cargo test transcribe_audio_with_vad -- --nocapture
    #[tokio::test(flavor = "multi_thread")]
    async fn transcribe_audio_with_vad() {
        let app = mock_builder()
            .plugin(tauri_plugin_shell::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build test app");
        let handle = app.handle().clone();

        let wav = format!("{}/tests/data/jfk.wav", env!("CARGO_MANIFEST_DIR"));

        let options = FrontendTranscribeOptions {
            audio_path: wav,
            offset: None,
            model: "moonshine-tiny".into(),
            lang: Some("en".into()),
            translate: Some(false),
            target_language: None,
            enable_dtw: Some(true),
            enable_gpu: Some(true),
            enable_diarize: Some(true),
            max_speakers: None,
            density: None,
            max_lines: None,
            custom_max_chars_per_line: None,
            text_case: None,
            remove_punctuation: None,
            censored_words: None,
            custom_prompt: None,
        };

        let res = transcribe_audio(handle, options).await;
        assert!(res.is_ok(), "VAD transcription failed: {:?}", res.err());

        if let Ok(transcript) = res {
            let out_path = format!(
                "{}/tests/data/transcript-vad.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let json = serde_json::to_string_pretty(&transcript)
                .expect("failed to serialize VAD transcript");
            fs::write(&out_path, json).expect("failed to write VAD transcript file");
            eprintln!("Saved VAD transcript to {}", out_path);
        }
    }

    // run with: cargo test transcribe_audio_qwen3_smoke -- --nocapture
    #[tokio::test(flavor = "multi_thread")]
    async fn transcribe_audio_qwen3_smoke() {
        unsafe {
            std::env::set_var(
                "HF_HOME",
                r"C:\Users\33287\AppData\Local\com.autosubs\models",
            );
            std::env::set_var(
                "HF_HUB_CACHE",
                r"C:\Users\33287\AppData\Local\com.autosubs\models",
            );
            std::env::set_var(
                "TRANSFORMERS_CACHE",
                r"C:\Users\33287\AppData\Local\com.autosubs\models",
            );
            std::env::set_var(
                "AUTOSUBS_MODEL_CACHE_DIR",
                r"C:\Users\33287\AppData\Local\com.autosubs\models",
            );
        }

        let app = mock_builder()
            .plugin(tauri_plugin_shell::init())
            .build(mock_context(noop_assets()))
            .expect("failed to build test app");
        let handle = app.handle().clone();

        let audio_path = r"D:\stream\rawB\2025-12-13 12-59-46.mkv".to_string();

        let options = FrontendTranscribeOptions {
            audio_path,
            offset: None,
            model: "qwen3-asr".into(),
            lang: Some("zh".into()),
            translate: Some(false),
            target_language: None,
            enable_dtw: Some(false),
            enable_gpu: Some(true),
            enable_diarize: Some(false),
            max_speakers: None,
            density: None,
            max_lines: None,
            custom_max_chars_per_line: None,
            text_case: None,
            remove_punctuation: None,
            censored_words: None,
            custom_prompt: None,
        };

        let res = transcribe_audio(handle, options).await;
        assert!(res.is_ok(), "qwen3 transcription failed: {:?}", res.err());

        if let Ok(transcript) = res {
            assert!(
                !transcript.segments.is_empty(),
                "qwen3 transcript returned no segments"
            );
            assert!(
                transcript.segments.iter().any(|segment| segment
                    .words
                    .as_ref()
                    .map(|words| !words.is_empty())
                    .unwrap_or(false)),
                "qwen3 transcript returned no word timestamps"
            );

            let out_path = format!(
                "{}/tests/data/transcript-qwen3-smoke.json",
                env!("CARGO_MANIFEST_DIR")
            );
            if let Some(parent) = std::path::Path::new(&out_path).parent() {
                fs::create_dir_all(parent)
                    .expect("failed to create qwen3 transcript output directory");
            }
            let json = serde_json::to_string_pretty(&transcript)
                .expect("failed to serialize qwen3 transcript");
            fs::write(&out_path, json).expect("failed to write qwen3 transcript file");
            eprintln!("Saved Qwen3 transcript to {}", out_path);
        }
    }
}
