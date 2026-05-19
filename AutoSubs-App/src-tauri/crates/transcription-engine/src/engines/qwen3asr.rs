//! Qwen3-ASR Python sidecar backend.

use crate::types::{
    LabeledProgressFn, NewSegmentFn, ProgressType, Segment, SpeechSegment, TranscribeOptions,
    WordTimestamp,
};
use eyre::{Context, Result, bail, eyre};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

const QWEN3_MODEL_NAME: &str = "qwen3-asr";
const QWEN_PROGRESS_PREFIX: &str = "AUTOSUBS_QWEN_PROGRESS ";

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

#[derive(Debug, Deserialize)]
struct QwenProgressEvent {
    progress: i32,
    label: String,
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

    emit_qwen_progress(progress_callback, 0, "progressSteps.qwenPreparing");

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
        .env("TRANSFORMERS_CACHE", cache_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to start Qwen3-ASR sidecar with {}",
            python.display()
        )
    })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre!("Failed to capture Qwen3-ASR stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre!("Failed to capture Qwen3-ASR stderr"))?;

    let mut diagnostics: Vec<String> = Vec::new();
    let mut stderr_lines = BufReader::new(stderr).lines();
    let stdout_task = tokio::spawn(async move {
        let mut stdout_bytes = Vec::new();
        stdout
            .read_to_end(&mut stdout_bytes)
            .await
            .context("Failed to read Qwen3-ASR stdout")?;
        Ok::<Vec<u8>, eyre::Report>(stdout_bytes)
    });

    while let Some(line) = stderr_lines
        .next_line()
        .await
        .context("Failed to read Qwen3-ASR stderr")?
    {
        handle_qwen_stderr_line(&line, progress_callback, &mut diagnostics);
    }

    let status = child
        .wait()
        .await
        .context("Failed to wait for Qwen3-ASR sidecar")?;
    let stdout_bytes = stdout_task
        .await
        .context("Qwen3-ASR stdout reader task failed")??;

    if let Some(is_cancelled) = abort_callback.as_deref() {
        if is_cancelled() {
            bail!("Transcription cancelled");
        }
    }

    if !status.success() {
        let stderr = diagnostics.join("\n");
        bail!(
            "Qwen3-ASR failed. Install Python 3.12 and `qwen-asr` first (`pip install -U qwen-asr`). stderr: {}",
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(stdout_bytes).context("Qwen3-ASR stdout was not valid UTF-8")?;
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

    emit_qwen_progress(progress_callback, 100, "progressSteps.transcribe");

    Ok((segments, qwen_output.language))
}

fn emit_qwen_progress(progress_callback: Option<&LabeledProgressFn>, progress: i32, label: &str) {
    if let Some(cb) = progress_callback {
        cb(progress, ProgressType::Transcribe, label);
    }
}

fn parse_qwen_progress_event(line: &str) -> Option<QwenProgressEvent> {
    let payload = line.strip_prefix(QWEN_PROGRESS_PREFIX)?;
    serde_json::from_str(payload).ok()
}

fn handle_qwen_stderr_line(
    line: &str,
    progress_callback: Option<&LabeledProgressFn>,
    diagnostics: &mut Vec<String>,
) {
    if let Some(event) = parse_qwen_progress_event(line) {
        tracing::info!("Qwen3-ASR progress: {}% {}", event.progress, event.label);
        emit_qwen_progress(progress_callback, event.progress.clamp(0, 99), &event.label);
    } else {
        tracing::info!("Qwen3-ASR sidecar: {}", line);
        diagnostics.push(line.to_string());
    }
}

fn qwen_device_arg(use_gpu: Option<bool>, gpu_device: Option<i32>) -> String {
    match use_gpu {
        Some(false) => "cpu".to_string(),
        _ => gpu_device
            .map(|device| format!("cuda:{}", device))
            .unwrap_or_else(|| "auto".to_string()),
    }
}

fn speaker_for_segment(
    start: f64,
    end: f64,
    diarized_segments: &[SpeechSegment],
) -> Option<String> {
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
    // 1. Check environment variable override
    if let Some(path) = std::env::var_os("AUTOSUBS_QWEN3_PYTHON") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 2. Check managed venv in app data directory
    // This is where the auto-installed Python environment lives
    if let Ok(data_dir) = std::env::var("LOCALAPPDATA") {
        let managed_python = PathBuf::from(&data_dir)
            .join("com.autosubs")
            .join("python")
            .join(".venv")
            .join(if cfg!(target_os = "windows") {
                "Scripts"
            } else {
                "bin"
            })
            .join(if cfg!(target_os = "windows") {
                "python.exe"
            } else {
                "python"
            });
        if managed_python.exists() {
            tracing::info!("using managed Python venv: {}", managed_python.display());
            return Ok(managed_python);
        }
    }

    // 3. Check local .venv candidates (for development)
    for candidate in local_python_candidates()? {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 4. Fallback to system Python
    Ok(PathBuf::from(if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    }))
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
        candidates.push(
            cwd.join("src-tauri")
                .join("resources")
                .join("qwen3_asr_transcribe.py"),
        );
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("resources").join("qwen3_asr_transcribe.py"));
            candidates.push(
                parent
                    .join("..")
                    .join("Resources")
                    .join("resources")
                    .join("qwen3_asr_transcribe.py"),
            );
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| eyre!("Unable to locate qwen3_asr_transcribe.py sidecar resource"))
}
