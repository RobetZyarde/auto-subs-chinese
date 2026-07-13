use eyre::{Result, bail};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use tauri_plugin_shell::ShellExt;
use tokio::process::Command as TokioCommand;

pub struct FfmpegOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub source: String,
}

fn legacy_windows_ffmpeg_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("AutoSubs").join("ffmpeg.exe"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn system_ffmpeg_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = std::env::var_os("AUTOSUBS_FFMPEG") {
        candidates.push(PathBuf::from(path));
    }

    candidates.push(PathBuf::from("ffmpeg"));

    if let Some(path) = legacy_windows_ffmpeg_path() {
        candidates.push(path);
    }

    candidates.dedup();
    candidates
}

pub async fn run_ffmpeg<R: Runtime>(app: &AppHandle<R>, args: &[String]) -> Result<FfmpegOutput> {
    let mut failures = Vec::new();

    match app.shell().sidecar("ffmpeg") {
        Ok(cmd) => match cmd.args(args.to_vec()).output().await {
            Ok(out) if out.status.success() => {
                return Ok(FfmpegOutput {
                    success: true,
                    code: out.status.code(),
                    stdout: out.stdout,
                    stderr: out.stderr,
                    source: "sidecar".to_string(),
                });
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                failures.push(format!(
                    "sidecar exited with code {:?}: {}",
                    out.status.code(),
                    stderr.trim()
                ));
            }
            Err(err) => {
                failures.push(format!("sidecar failed to start: {err}"));
            }
        },
        Err(err) => {
            failures.push(format!("sidecar failed to initialize: {err}"));
        }
    }

    for candidate in system_ffmpeg_candidates() {
        match TokioCommand::new(&candidate).args(args).output().await {
            Ok(out) if out.status.success() => {
                return Ok(FfmpegOutput {
                    success: true,
                    code: out.status.code(),
                    stdout: out.stdout,
                    stderr: out.stderr,
                    source: candidate.to_string_lossy().to_string(),
                });
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                failures.push(format!(
                    "{} exited with code {:?}: {}",
                    candidate.display(),
                    out.status.code(),
                    stderr.trim()
                ));
            }
            Err(err) => {
                failures.push(format!("{} failed to start: {err}", candidate.display()));
            }
        }
    }

    bail!("ffmpeg unavailable: {}", failures.join("; "))
}
