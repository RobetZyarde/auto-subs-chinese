//! Qwen3-ASR Python sidecar backend.

use crate::types::{LabeledProgressFn, NewSegmentFn, ProgressType, Segment, SpeechSegment, TranscribeOptions, WordTimestamp};
use eyre::{bail, eyre, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

const QWEN3_MODEL_NAME: &str = "qwen3-asr";

#[derive(Debug, Deserialize)]
struct Qwen3Word {
    text: String,
    start: f64,
    end: f64,
    #[serde(default)]
    probability: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Qwen3Segment {
    start: f64,
    end: f64,
    text: String,
    #[serde(default)]
    words: Vec<Qwen3Word>,
    #[serde(default)]
    speaker_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Qwen3Output {
    segments: Vec<Qwen3Segment>,
    language: Option<String>,
}

pub fn is_qwen3asr_model(model_name: &str) -> bool {
    model_name.eq_ignore_ascii_case(QWEN3_MODEL_NAME)
}

pub async fn transcribe_qwen3asr(
    audio_path: &Path,
    diarized_segments: &[SpeechSegment],
    options: &TranscribeOptions,
    cache_dir: &Path,
    use_gpu: Option<bool>,
    gpu_device: Option<i32>,
    progress_callback: Option<&LabeledProgressFn>,
    new_segment_callback: Option<&NewSegmentFn>,
    abort_callback: Option<Box<dyn Fn() -> bool + Send + Sync>>,
) -> Result<(Vec<Segment>, Option<String>)> {
    if let Some(is_cancelled) = abort_callback.as_deref() {
        if is_cancelled() {
            bail!("Transcription cancelled");
        }
    }

    if let Some(cb) = progress_callback {
        cb(0, ProgressType::Transcribe, "progressSteps.transcribe");
    }

    let python = find_python()?;
    let script = find_sidecar_script()?;
    let device = qwen_device_arg(use_gpu, gpu_device);

    let mut cmd = Command::new(&python);
    cmd.arg(&script)
        .arg(audio_path)
        .arg("--language")
        .arg(options.lang.as_deref().unwrap_or("auto"))
        .arg("--device")
        .arg(device)
        .env("HF_HOME", cache_dir)
        .env("HF_HUB_CACHE", cache_dir)
        .env("TRANSFORMERS_CACHE", cache_dir);

    if let Some(context) = options
        .advanced
        .as_ref()
        .and_then(|advanced| advanced.init_prompt.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        cmd.arg("--context").arg(context);
    }

    tracing::info!(
        "starting Qwen3-ASR sidecar: python={}, script={}, audio={}",
        python.display(),
        script.display(),
        audio_path.display()
    );

    let output = cmd
        .output()
        .await
        .with_context(|| format!("Failed to start Qwen3-ASR sidecar with {}", python.display()))?;

    if let Some(is_cancelled) = abort_callback.as_deref() {
        if is_cancelled() {
            bail!("Transcription cancelled");
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Qwen3-ASR failed. Install Python 3.12 and `qwen-asr` first (`pip install -U qwen-asr`). stderr: {}",
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("Qwen3-ASR stdout was not valid UTF-8")?;
    let qwen_output: Qwen3Output = serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse Qwen3-ASR JSON output: {}", stdout.trim()))?;

    let user_offset = options.offset.unwrap_or(0.0);
    let mut segments = Vec::with_capacity(qwen_output.segments.len());
    for segment in qwen_output.segments {
        let start = segment.start + user_offset;
        let end = segment.end + user_offset;
        let speaker_id = segment
            .speaker_id
            .or_else(|| speaker_for_segment(start, end, diarized_segments));

        let words: Vec<WordTimestamp> = segment
            .words
            .into_iter()
            .map(|word| WordTimestamp {
                text: word.text,
                start: word.start + user_offset,
                end: word.end + user_offset,
                probability: word.probability,
            })
            .collect();

        let autosubs_segment = Segment {
            start,
            end,
            text: segment.text,
            words: (!words.is_empty()).then_some(words),
            speaker_id,
        };

        if let Some(cb) = new_segment_callback {
            cb(&autosubs_segment);
        }
        segments.push(autosubs_segment);
    }

    if let Some(cb) = progress_callback {
        cb(100, ProgressType::Transcribe, "progressSteps.transcribe");
    }

    Ok((segments, qwen_output.language))
}

fn qwen_device_arg(use_gpu: Option<bool>, gpu_device: Option<i32>) -> String {
    match use_gpu {
        Some(false) => "cpu".to_string(),
        _ => gpu_device
            .map(|device| format!("cuda:{}", device))
            .unwrap_or_else(|| "auto".to_string()),
    }
}

fn speaker_for_segment(start: f64, end: f64, diarized_segments: &[SpeechSegment]) -> Option<String> {
    if diarized_segments.is_empty() {
        return None;
    }
    let midpoint = start + ((end - start).max(0.0) / 2.0);
    diarized_segments
        .iter()
        .find(|segment| midpoint >= segment.start && midpoint <= segment.end)
        .and_then(|segment| segment.speaker_id.clone())
}

fn find_python() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("AUTOSUBS_QWEN3_PYTHON") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    for candidate in local_python_candidates()? {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(PathBuf::from(if cfg!(target_os = "windows") { "python" } else { "python3" }))
}

fn local_python_candidates() -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    let mut dirs = Vec::new();
    dirs.push(std::env::current_dir().context("Failed to read current directory")?);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    for dir in dirs {
        for ancestor in dir.ancestors().take(6) {
            if cfg!(target_os = "windows") {
                candidates.push(ancestor.join(".venv").join("Scripts").join("python.exe"));
            } else {
                candidates.push(ancestor.join(".venv").join("bin").join("python"));
            }
        }
    }
    Ok(candidates)
}

fn find_sidecar_script() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("AUTOSUBS_QWEN3_ASR_SCRIPT") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("resources").join("qwen3_asr_transcribe.py"));
        candidates.push(cwd.join("src-tauri").join("resources").join("qwen3_asr_transcribe.py"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("resources").join("qwen3_asr_transcribe.py"));
            candidates.push(parent.join(".." ).join("Resources").join("resources").join("qwen3_asr_transcribe.py"));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| eyre!("Unable to locate qwen3_asr_transcribe.py sidecar resource"))
}
