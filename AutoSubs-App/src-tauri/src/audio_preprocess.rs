use eyre::{Result, bail};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

/// Converts audio/video files to mono 16kHz 16-bit PCM WAV using FFmpeg.
/// This is the only preprocessing step needed before passing audio to whisper-diarize-rs.
/// Handles both audio files and video files (extracts audio stream only).
pub async fn normalize<R: Runtime>(
    app: AppHandle<R>,
    input: PathBuf,
    output: PathBuf,
    additional_ffmpeg_args: Option<Vec<String>>,
) -> std::result::Result<(), String> {
    async fn normalize_inner<R: Runtime>(
        app: &AppHandle<R>,
        input: PathBuf,
        output: PathBuf,
        additional_ffmpeg_args: Option<Vec<String>>,
    ) -> Result<()> {
        // Ensure the output directory exists
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        tracing::info!(
            "audio normalization: converting to mono 16kHz PCM16 WAV ({} -> {})",
            input.display(),
            output.display()
        );

        let input_lossy = input.to_string_lossy().into_owned();
        let output_lossy = output.to_string_lossy().into_owned();

        // Build FFmpeg command: extract audio, convert to mono 16kHz PCM16 WAV
        let mut args = vec![
            "-nostdin".into(),
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-vn".into(), // No video
            "-sn".into(), // No subtitles
            "-dn".into(), // No data streams
            "-i".into(),
            input_lossy,
            "-ar".into(),
            "16000".into(), // Sample rate: 16kHz
            "-ac".into(),
            "1".into(), // Channels: mono
            "-c:a".into(),
            "pcm_s16le".into(), // Codec: 16-bit PCM
            "-map_metadata".into(),
            "-1".into(), // Strip metadata
            "-f".into(),
            "wav".into(), // Format: WAV
            "-nostats".into(),
        ];

        // Add any additional FFmpeg arguments
        if let Some(ref additional_args) = additional_ffmpeg_args {
            args.extend(additional_args.clone());
        }

        // Overwrite output file
        args.push("-y".into());
        args.push(output_lossy);

        tracing::debug!("Running ffmpeg with args: {:?}", args);

        let ffmpeg_output = crate::ffmpeg::run_ffmpeg(app, &args).await?;
        tracing::debug!("ffmpeg source: {}", ffmpeg_output.source);

        // Log FFmpeg output for diagnostics
        if !ffmpeg_output.stdout.is_empty() {
            tracing::debug!(
                "ffmpeg stdout: {}",
                String::from_utf8_lossy(&ffmpeg_output.stdout)
            );
        }
        if !ffmpeg_output.stderr.is_empty() {
            tracing::debug!(
                "ffmpeg stderr: {}",
                String::from_utf8_lossy(&ffmpeg_output.stderr)
            );
        }

        // Check for errors
        if !ffmpeg_output.success {
            let error_message = String::from_utf8_lossy(&ffmpeg_output.stderr);
            bail!(
                "ffmpeg failed with exit code: {:?}\nStderr: {}",
                ffmpeg_output.code,
                error_message
            );
        }

        // Verify output file was created
        if !output.exists() {
            bail!("ffmpeg succeeded but output file was not created");
        }

        // Check file size
        let out_meta = fs::metadata(&output)?;
        if out_meta.len() <= 44 {
            tracing::warn!(
                "Output WAV file is suspiciously small (header-only): {:?}",
                output
            );
        }

        tracing::info!("audio normalization: success -> {}", output.display());
        Ok(())
    }

    normalize_inner(&app, input, output, additional_ffmpeg_args)
        .await
        .map_err(|e| e.to_string())
}
