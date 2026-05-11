//! Python environment management for Qwen3-ASR sidecar.
//!
//! Creates a virtual environment in the app data directory and installs
//! the `qwen-asr` package on first use.

use eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::process::Command;

/// Status of the Python environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelState")]
pub enum PythonEnvStatus {
    /// Environment is ready with qwen-asr installed.
    Ready,
    /// Environment needs to be created or dependencies installed.
    NotReady,
    /// Installation is in progress.
    Installing,
    /// Installation failed.
    Failed(String),
}

/// Progress update emitted to the frontend during installation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonInstallProgress {
    pub stage: String,
    pub percent: i32,
    pub message: String,
}

/// Get the path to the managed Python virtual environment.
pub fn get_venv_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let data_dir = app.path().app_data_dir().unwrap_or_else(|_| {
        // Fallback to LOCALAPPDATA
        let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(local).join("com.autosubs")
    });
    data_dir.join("python").join(".venv")
}

/// Get the path to the Python executable in the managed venv.
pub fn get_venv_python<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let venv = get_venv_dir(app);
    if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

/// Get the path to the pip executable in the managed venv.
fn get_venv_pip<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    let venv = get_venv_dir(app);
    if cfg!(target_os = "windows") {
        venv.join("Scripts").join("pip.exe")
    } else {
        venv.join("bin").join("pip")
    }
}

/// Check if the managed Python environment exists and has qwen-asr installed.
pub async fn check_env_status<R: Runtime>(app: &AppHandle<R>) -> PythonEnvStatus {
    let python = get_venv_python(app);
    if !python.exists() {
        return PythonEnvStatus::NotReady;
    }

    // Check if qwen_asr is importable
    let output = Command::new(&python)
        .args(["-c", "import qwen_asr; print('ok')"])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => PythonEnvStatus::Ready,
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::debug!("qwen_asr check failed: {}", stderr);
            PythonEnvStatus::NotReady
        }
        Err(e) => {
            tracing::debug!("Python check failed: {}", e);
            PythonEnvStatus::NotReady
        }
    }
}

/// Find a system Python executable that supports creating venvs.
async fn find_system_python() -> Result<PathBuf> {
    // Try python3 first, then python
    for name in &["python3", "python"] {
        if let Ok(output) = Command::new(name).args(["--version"]).output().await {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                tracing::info!("found system Python: {} - {}", name, version.trim());
                return Ok(PathBuf::from(name));
            }
        }
    }

    // On Windows, try py launcher
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("py")
            .args(["-3.12", "--version"])
            .output()
            .await
        {
            if output.status.success() {
                tracing::info!("found Python via py launcher");
                return Ok(PathBuf::from("py"));
            }
        }
    }

    bail!("Python not found. Please install Python 3.10+ and ensure it's in your PATH.")
}

/// Emit a progress event to the frontend.
fn emit_progress<R: Runtime>(app: &AppHandle<R>, stage: &str, percent: i32, message: &str) {
    let _ = app.emit(
        "python-install-progress",
        PythonInstallProgress {
            stage: stage.to_string(),
            percent,
            message: message.to_string(),
        },
    );
}

/// Check Python environment status (Tauri command).
#[tauri::command]
pub async fn check_python_env<R: Runtime>(app: AppHandle<R>) -> Result<PythonEnvStatus, String> {
    let python = get_venv_python(&app);
    if !python.exists() {
        return Ok(PythonEnvStatus::NotReady);
    }
    let output = Command::new(&python)
        .args(["-c", "import qwen_asr; print('ok')"])
        .output()
        .await;
    match output {
        Ok(out) if out.status.success() => Ok(PythonEnvStatus::Ready),
        _ => Ok(PythonEnvStatus::NotReady),
    }
}

/// Ensure the Python environment is set up with qwen-asr installed (Tauri command).
#[tauri::command]
pub async fn ensure_python_env<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    ensure_python_env_impl(app).await.map_err(|e| e.to_string())
}



/// Internal implementation for ensure_python_env.
async fn ensure_python_env_impl<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    ensure_python_env_inner(app).await
}

async fn ensure_python_env_inner<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    let venv_dir = get_venv_dir(&app);
    let python_path = get_venv_python(&app);
    let pip_path = get_venv_pip(&app);

    // Check if already ready
    let status = check_env_status(&app).await;
    if status == PythonEnvStatus::Ready {
        tracing::info!("Python environment already ready");
        let _ = app.emit(
            "python-install-progress",
            PythonInstallProgress {
                stage: "done".to_string(),
                percent: 100,
                message: "Python environment is ready".to_string(),
            },
        );
        return Ok(());
    }

    emit_progress(&app, "preparing", 0, "Finding Python installation...");

    // Find system Python
    let system_python = find_system_python().await?;

    // Create parent directory
    if let Some(parent) = venv_dir.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create python directory")?;
    }

    // Remove existing venv if it exists but is broken
    if venv_dir.exists() {
        emit_progress(&app, "preparing", 5, "Removing broken environment...");
        tokio::fs::remove_dir_all(&venv_dir)
            .await
            .context("Failed to remove existing venv")?;
    }

    // Step 1: Create virtual environment
    emit_progress(&app, "creating_venv", 10, "Creating virtual environment...");

    let mut create_cmd = if cfg!(target_os = "windows") && system_python.to_str() == Some("py") {
        let mut cmd = Command::new("py");
        cmd.args(["-3.12", "-m", "venv"]);
        cmd
    } else {
        let mut cmd = Command::new(&system_python);
        cmd.args(["-m", "venv"]);
        cmd
    };

    let output = create_cmd
        .arg(&venv_dir)
        .output()
        .await
        .context("Failed to run python -m venv")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to create virtual environment: {}", stderr);
    }

    tracing::info!("created venv at {}", venv_dir.display());
    emit_progress(&app, "creating_venv", 30, "Virtual environment created");

    // Step 2: Upgrade pip
    emit_progress(&app, "upgrading_pip", 35, "Upgrading pip...");

    let output = Command::new(&pip_path)
        .args(["install", "--upgrade", "pip"])
        .output()
        .await
        .context("Failed to run pip upgrade")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("pip upgrade failed (continuing anyway): {}", stderr);
    }

    emit_progress(&app, "upgrading_pip", 45, "Pip upgraded");

    // Step 3: Install qwen-asr
    emit_progress(
        &app,
        "installing_qwen_asr",
        50,
        "Installing qwen-asr (this may take a few minutes)...",
    );

    let output = Command::new(&pip_path)
        .args(["install", "-U", "qwen-asr"])
        .output()
        .await
        .context("Failed to install qwen-asr")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to install qwen-asr: {}", stderr);
    }

    tracing::info!("installed qwen-asr");
    emit_progress(&app, "installing_qwen_asr", 90, "qwen-asr installed");

    // Step 4: Verify installation
    emit_progress(&app, "verifying", 95, "Verifying installation...");

    let output = Command::new(&python_path)
        .args(["-c", "from qwen_asr import Qwen3ASRModel; print('ok')"])
        .output()
        .await
        .context("Failed to verify qwen-asr installation")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("qwen-asr verification failed: {}", stderr);
    }

    emit_progress(&app, "done", 100, "Python environment is ready!");

    tracing::info!("Python environment setup complete");
    Ok(())
}

/// Install CUDA-enabled PyTorch for GPU acceleration.
#[tauri::command]
pub async fn install_cuda_pytorch<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    install_cuda_pytorch_impl(app).await.map_err(|e| e.to_string())
}

/// Internal implementation for install_cuda_pytorch.
async fn install_cuda_pytorch_impl<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    install_cuda_pytorch_inner(app).await
}

/// Install CUDA PyTorch inner implementation.
async fn install_cuda_pytorch_inner<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    let pip_path = get_venv_pip(&app);

    emit_progress(&app, "installing_cuda", 10, "Uninstalling CPU PyTorch...");

    // Uninstall existing torch
    let output = Command::new(&pip_path)
        .args(["uninstall", "-y", "torch", "torchvision", "torchaudio"])
        .output()
        .await
        .context("Failed to uninstall torch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("torch uninstall warning: {}", stderr);
    }

    emit_progress(
        &app,
        "installing_cuda",
        30,
        "Installing CUDA PyTorch (this may take a while)...",
    );

    // Install CUDA version
    let output = Command::new(&pip_path)
        .args([
            "install",
            "--upgrade",
            "--force-reinstall",
            "torch",
            "torchvision",
            "torchaudio",
            "--index-url",
            "https://download.pytorch.org/whl/cu128",
        ])
        .output()
        .await
        .context("Failed to install CUDA PyTorch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to install CUDA PyTorch: {}", stderr);
    }

    emit_progress(&app, "installing_cuda", 80, "Verifying CUDA support...");

    let output = Command::new(get_venv_python(&app))
        .args(["-c", "import torch; print(torch.cuda.is_available())"])
        .output()
        .await
        .context("Failed to verify CUDA")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cuda_available = stdout.trim() == "True";

    if cuda_available {
        emit_progress(&app, "done", 100, "CUDA PyTorch installed successfully!");
    } else {
        emit_progress(
            &app,
            "done",
            100,
            "PyTorch installed but CUDA not available. Will use CPU.",
        );
    }

    Ok(())
}
