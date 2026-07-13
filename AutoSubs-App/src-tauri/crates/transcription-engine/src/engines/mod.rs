//! Transcription engine backends.
//!
//! This module provides different speech recognition backends:
//! - **Whisper**: OpenAI's Whisper model via whisper-rs (GGML format)
//! - **Parakeet**: NVIDIA's NeMo Parakeet model via transcribe-rs (ONNX format)
//! - **Moonshine**: Useful Sensors' Moonshine via transcribe-rs (ONNX format)
//! - **SenseVoice**: FunAudioLLM SenseVoice via transcribe-rs (ONNX format)

use crate::engine::EngineConfig;
use crate::types::{LabeledProgressFn, NewSegmentFn, Segment, SpeechSegment, TranscribeOptions};
use crate::manifest::Engine as ModelEngine;
use eyre::{eyre, Result};
use std::path::Path;
use transcribe_rs::{set_ort_accelerator, OrtAccelerator};

pub mod whisper;

pub mod onnx;
pub mod canary;
pub mod cohere;
pub mod moonshine;
pub mod parakeet;
pub mod qwen3asr;
pub mod sense_voice;

// Re-export commonly used items
pub use whisper::{create_context, run_transcription_pipeline, SHOULD_CANCEL};

pub use canary::transcribe_canary;
pub use cohere::transcribe_cohere;
pub use moonshine::transcribe_moonshine;
pub use parakeet::transcribe_parakeet;
pub use sense_voice::transcribe_sense_voice;

fn ort_accelerator_for(use_gpu: Option<bool>) -> OrtAccelerator {
    if use_gpu == Some(false) {
        return OrtAccelerator::CpuOnly;
    }

    #[cfg(feature = "directml")]
    return OrtAccelerator::DirectMl;

    #[cfg(all(not(feature = "directml"), feature = "coreml"))]
    return OrtAccelerator::CoreMl;

    #[cfg(not(any(feature = "directml", feature = "coreml")))]
    OrtAccelerator::Auto
}

fn configure_ort_accelerator(use_gpu: Option<bool>) -> OrtAccelerator {
    let accelerator = ort_accelerator_for(use_gpu);
    set_ort_accelerator(accelerator);
    accelerator
}

#[allow(clippy::too_many_arguments)]
pub async fn run_engine(
    engine_kind: ModelEngine,
    audio_path: &Path,
    model_path: &Path,
    speech_segments: Vec<SpeechSegment>,
    options: &TranscribeOptions,
    native_target: Option<&str>,
    cfg: &EngineConfig,
    progress: Option<&LabeledProgressFn>,
    new_segment_callback: Option<&NewSegmentFn>,
    abort_callback: Option<Box<dyn Fn() -> bool + Send + Sync>>,
) -> Result<(Vec<Segment>, Option<String>)> {
    let num_samples: usize = speech_segments.iter().map(|s| s.samples.len()).sum();

    if matches!(
        engine_kind,
        ModelEngine::Parakeet
            | ModelEngine::Moonshine
            | ModelEngine::Canary
            | ModelEngine::Cohere
            | ModelEngine::Gigaam
            | ModelEngine::SenseVoice
    ) {
        let accelerator = configure_ort_accelerator(cfg.use_gpu);
        tracing::info!(
            "ONNX: loading model with accelerator={} (use_gpu={:?})",
            accelerator,
            cfg.use_gpu
        );
    }

    match engine_kind {
        ModelEngine::Parakeet => {
            crate::engines::parakeet::transcribe_parakeet(
                model_path,
                speech_segments,
                options,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::Moonshine => {
            let (variant, _lang) = crate::engines::moonshine::moonshine_variant_from_model_name(&options.model)
                .ok_or_else(|| eyre!("Unknown Moonshine model: {}", options.model))?;

            crate::engines::moonshine::transcribe_moonshine(
                model_path,
                variant,
                speech_segments,
                options,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::Whisper => {
            tracing::info!(
                "Whisper: loading model context (model={}, use_gpu={:?})",
                options.model,
                cfg.use_gpu
            );
            let ctx_start = std::time::Instant::now();
            let ctx = crate::engines::whisper::create_context(
                model_path,
                &options.model,
                cfg.gpu_device,
                cfg.use_gpu,
                cfg.enable_dtw,
                cfg.enable_flash_attn,
                Some(num_samples),
            )
            .map_err(|e| eyre!("Failed to create Whisper context: {}", e))?;
            tracing::info!(
                "Whisper: model context ready in {:.2}s",
                ctx_start.elapsed().as_secs_f64()
            );

            crate::engines::whisper::run_transcription_pipeline(
                ctx,
                speech_segments,
                options.clone(),
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::SenseVoice => {
            crate::engines::sense_voice::transcribe_sense_voice(
                model_path,
                speech_segments,
                options,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::Canary => {
            crate::engines::canary::transcribe_canary(
                model_path,
                speech_segments,
                options,
                native_target,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::Cohere => {
            crate::engines::cohere::transcribe_cohere(
                model_path,
                speech_segments,
                options,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        ModelEngine::Qwen3Asr => {
            crate::engines::qwen3asr::transcribe_qwen3asr(
                audio_path,
                &speech_segments,
                options,
                model_path,
                cfg.use_gpu,
                cfg.gpu_device,
                progress,
                new_segment_callback,
                abort_callback,
            )
            .await
        }
        other => Err(eyre!(
            "Transcription engine {:?} (model '{}') is not yet supported",
            other,
            options.model
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabling_gpu_forces_ort_cpu() {
        assert_eq!(ort_accelerator_for(Some(false)), OrtAccelerator::CpuOnly);
    }

    #[cfg(feature = "directml")]
    #[test]
    fn windows_gpu_uses_directml_for_ort_models() {
        assert_eq!(ort_accelerator_for(Some(true)), OrtAccelerator::DirectMl);
    }

    #[cfg(all(not(feature = "directml"), feature = "coreml"))]
    #[test]
    fn mac_gpu_uses_coreml_for_ort_models() {
        assert_eq!(ort_accelerator_for(Some(true)), OrtAccelerator::CoreMl);
    }
}
