#!/usr/bin/env python3
"""Qwen3-ASR sidecar for AutoSubs.

The Rust app invokes this script as a subprocess and reads a single JSON object
from stdout. Diagnostics must go to stderr so stdout stays parseable.
"""

from __future__ import annotations

import argparse
import gc
import importlib.util
import json
import os
import sys
import wave
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


ASR_MODEL = "Qwen/Qwen3-ASR-1.7B"
ALIGNER_MODEL = "Qwen/Qwen3-ForcedAligner-0.6B"

LANGUAGE_TO_QWEN = {
    "zh": "Chinese",
    "zh-cn": "Chinese",
    "zh-tw": "Chinese",
    "en": "English",
    "yue": "Cantonese",
    "fr": "French",
    "de": "German",
    "it": "Italian",
    "ja": "Japanese",
    "ko": "Korean",
    "pt": "Portuguese",
    "ru": "Russian",
    "es": "Spanish",
    "ar": "Arabic",
    "id": "Indonesian",
    "th": "Thai",
    "vi": "Vietnamese",
    "tr": "Turkish",
    "hi": "Hindi",
    "ms": "Malay",
    "nl": "Dutch",
    "sv": "Swedish",
    "da": "Danish",
    "fi": "Finnish",
    "pl": "Polish",
    "cs": "Czech",
    "fil": "Filipino",
    "fa": "Persian",
    "el": "Greek",
    "hu": "Hungarian",
    "mk": "Macedonian",
    "ro": "Romanian",
}

QWEN_TO_LANGUAGE = {value.lower(): key for key, value in LANGUAGE_TO_QWEN.items()}
QWEN_TO_LANGUAGE.update({"chinese": "zh", "cantonese": "yue", "english": "en"})

CJK_LANGUAGES = {"Chinese", "Cantonese", "Japanese", "Korean"}
SENTENCE_ENDINGS = set("。！？.!?\n")
SEGMENT_GAP_SECONDS = 0.45
PROGRESS_PREFIX = "AUTOSUBS_QWEN_PROGRESS "
RUNTIME_PREFIX = "AUTOSUBS_QWEN_RUNTIME "
CLEANUP_PREFIX = "AUTOSUBS_QWEN_CLEANUP "


def should_use_flash_attention() -> bool:
    """Enable FlashAttention-2 when it is installed, with an env override.

    AUTOSUBS_QWEN_FLASH_ATTN=1 forces it on and lets Transformers report a
    precise dependency/runtime error. AUTOSUBS_QWEN_FLASH_ATTN=0 disables it.
    The default is opportunistic so the sidecar keeps working on Windows
    environments that cannot install flash-attn yet.
    """
    value = os.environ.get("AUTOSUBS_QWEN_FLASH_ATTN", "auto").strip().lower()
    if value in {"0", "false", "no", "off", "disabled"}:
        return False
    if value in {"1", "true", "yes", "on", "enabled"}:
        return True
    return importlib.util.find_spec("flash_attn") is not None


@dataclass
class TimestampItem:
    text: str
    start: float
    end: float


def emit_progress(progress: int, label: str) -> None:
    payload = {"progress": progress, "label": label}
    print(f"{PROGRESS_PREFIX}{json.dumps(payload, ensure_ascii=False)}", file=sys.stderr, flush=True)


def emit_runtime(payload: dict[str, Any]) -> None:
    print(f"{RUNTIME_PREFIX}{json.dumps(payload, ensure_ascii=False)}", file=sys.stderr, flush=True)


def emit_cleanup(payload: dict[str, Any]) -> None:
    print(f"{CLEANUP_PREFIX}{json.dumps(payload, ensure_ascii=False)}", file=sys.stderr, flush=True)


def audio_duration_seconds(audio_path: str) -> float:
    try:
        with wave.open(audio_path, "rb") as wav:
            frames = wav.getnframes()
            rate = wav.getframerate()
            return frames / rate if rate else 0.0
    except Exception:
        return 0.0


def normalize_language_for_qwen(language: str | None) -> str | None:
    if not language or language.lower() == "auto":
        return None
    return LANGUAGE_TO_QWEN.get(language.lower(), language)


def normalize_language_for_autosubs(language: Any, requested: str | None) -> str:
    if isinstance(language, str) and language.strip():
        normalized = QWEN_TO_LANGUAGE.get(language.strip().lower())
        if normalized:
            return normalized
    if requested and requested.lower() != "auto":
        return requested
    return "auto"


def get_field(value: Any, *names: str) -> Any:
    if isinstance(value, dict):
        for name in names:
            if name in value:
                return value[name]
    for name in names:
        if hasattr(value, name):
            return getattr(value, name)
    return None


def normalize_time(value: Any) -> float | None:
    if value is None:
        return None
    try:
        time_value = float(value)
    except (TypeError, ValueError):
        return None
    # Some timestamp APIs expose milliseconds. Qwen examples use seconds.
    if time_value > 10_000:
        return time_value / 1000.0
    return time_value


def flatten_timestamp_items(raw: Any) -> Iterable[Any]:
    if raw is None:
        return
    items = get_field(raw, "items")
    if items is not None:
        yield from flatten_timestamp_items(items)
        return
    if isinstance(raw, (list, tuple)):
        for item in raw:
            yield from flatten_timestamp_items(item)
        return
    yield raw


def extract_timestamps(raw: Any) -> list[TimestampItem]:
    timestamps: list[TimestampItem] = []
    for item in flatten_timestamp_items(raw):
        text = get_field(item, "text", "word", "token")
        start = normalize_time(get_field(item, "start_time", "start", "start_s"))
        end = normalize_time(get_field(item, "end_time", "end", "end_s"))
        if text is None or start is None or end is None:
            continue
        text = str(text)
        if not text.strip():
            continue
        timestamps.append(TimestampItem(text=text, start=start, end=end))
    timestamps.sort(key=lambda item: (item.start, item.end))
    return timestamps


def merge_transcript_text(
    transcript_text: str,
    timestamps: list[TimestampItem],
) -> tuple[list[TimestampItem], str]:
    if not transcript_text or not timestamps:
        return timestamps, transcript_text.strip()

    merged: list[TimestampItem] = []
    cursor = 0

    def attach_prefix(prefix: str) -> None:
        nonlocal merged
        if not prefix:
            return
        if merged:
            last = merged[-1]
            merged[-1] = TimestampItem(text=last.text + prefix, start=last.start, end=last.end)

    for item in timestamps:
        token = item.text
        prefix = ""

        while cursor < len(transcript_text) and transcript_text[cursor] != token[:1]:
            prefix += transcript_text[cursor]
            cursor += 1

        attach_prefix(prefix)
        prefix = ""

        if cursor < len(transcript_text) and transcript_text[cursor : cursor + len(token)] == token:
            token = transcript_text[cursor : cursor + len(token)]
            cursor += len(item.text)

        merged.append(TimestampItem(text=token, start=item.start, end=item.end))

    trailing = transcript_text[cursor:].strip()
    if trailing and merged:
        last = merged[-1]
        merged[-1] = TimestampItem(text=last.text + trailing, start=last.start, end=last.end)

    return merged, transcript_text.strip()


def should_insert_space(previous: str, text: str, qwen_language: str | None) -> bool:
    if not previous or not text or qwen_language in CJK_LANGUAGES:
        return False
    if text[0].isspace() or previous[-1].isspace():
        return False
    return previous[-1].isalnum() and text[0].isalnum()


def build_segments(
    text: str,
    timestamps: list[TimestampItem],
    qwen_language: str | None,
    duration: float,
) -> list[dict[str, Any]]:
    if not timestamps:
        cleaned = text.strip()
        return [{"start": 0.0, "end": duration, "text": cleaned, "words": []}] if cleaned else []

    segments: list[dict[str, Any]] = []
    current_words: list[dict[str, Any]] = []
    current_text = ""
    current_start: float | None = None
    current_end = 0.0

    def flush() -> None:
        nonlocal current_words, current_text, current_start, current_end
        cleaned = current_text.strip()
        if cleaned and current_start is not None:
            segments.append(
                {
                    "start": current_start,
                    "end": current_end,
                    "text": cleaned,
                    "words": current_words,
                }
            )
        current_words = []
        current_text = ""
        current_start = None
        current_end = 0.0

    for item in timestamps:
        if current_start is not None and (item.start - current_end) >= SEGMENT_GAP_SECONDS:
            flush()

        token_text = item.text
        if should_insert_space(current_text, token_text, qwen_language):
            token_text = " " + token_text
        if current_start is None:
            current_start = item.start
        current_text += token_text
        current_end = item.end
        current_words.append({"text": token_text, "start": item.start, "end": item.end})

        if token_text[-1:] in SENTENCE_ENDINGS or len(current_text) >= 80:
            flush()

    flush()
    return segments


def transcribe(args: argparse.Namespace) -> dict[str, Any]:
    emit_progress(5, "progressSteps.qwenPreparing")

    torch_module = None
    model = None
    result = None
    results = None
    result_payload: dict[str, Any] | None = None
    cleanup: dict[str, Any] = {}
    try:
        import torch as torch_module
        from qwen_asr import Qwen3ASRModel

        audio_path = str(Path(args.audio_path).resolve())
        qwen_language = normalize_language_for_qwen(args.language)
        device_map = args.device if args.device != "auto" else ("cuda:0" if torch_module.cuda.is_available() else "cpu")
        use_cuda = str(device_map).startswith("cuda")
        dtype = torch_module.bfloat16 if use_cuda else torch_module.float32
        flash_attention_requested = should_use_flash_attention()
        flash_attention_enabled = use_cuda and flash_attention_requested

        model_kwargs: dict[str, Any] = {
            "dtype": dtype,
            "device_map": device_map,
            "max_inference_batch_size": args.max_batch_size,
            "max_new_tokens": args.max_new_tokens,
        }
        if flash_attention_enabled:
            model_kwargs["attn_implementation"] = "flash_attention_2"
        if args.alignment:
            model_kwargs["forced_aligner"] = ALIGNER_MODEL
            model_kwargs["forced_aligner_kwargs"] = {"dtype": dtype, "device_map": device_map}

        runtime = {
            "device_map": device_map,
            "dtype": str(dtype).replace("torch.", ""),
            "cuda_available": bool(torch_module.cuda.is_available()),
            "cuda_device_name": torch_module.cuda.get_device_name(0) if torch_module.cuda.is_available() else None,
            "flash_attn_importable": importlib.util.find_spec("flash_attn") is not None,
            "flash_attention_requested": bool(flash_attention_requested),
            "flash_attention_enabled": bool(flash_attention_enabled),
            "attn_implementation": model_kwargs.get("attn_implementation"),
            "forced_aligner": ALIGNER_MODEL if args.alignment else None,
        }
        emit_runtime(runtime)

        emit_progress(15, "progressSteps.qwenLoadingModel")
        model = Qwen3ASRModel.from_pretrained(ASR_MODEL, **model_kwargs)

        emit_progress(35, "progressSteps.qwenInferencing")
        results = model.transcribe(
            audio=audio_path,
            context=args.context or None,
            language=qwen_language,
            return_time_stamps=args.alignment,
        )
        result = results[0] if isinstance(results, (list, tuple)) else results

        emit_progress(85, "progressSteps.qwenPostProcessing")
        result_text = str(get_field(result, "text") or "")
        result_language = get_field(result, "language")
        timestamps = extract_timestamps(get_field(result, "time_stamps", "timestamps"))
        timestamps, result_text = merge_transcript_text(result_text, timestamps)
        duration = audio_duration_seconds(audio_path)
        segments = build_segments(result_text, timestamps, qwen_language, duration)

        result_payload = {
            "segments": segments,
            "language": normalize_language_for_autosubs(result_language, args.language),
            "model": ASR_MODEL,
            "runtime": runtime,
        }
    finally:
        cleanup = {"model_loaded": model is not None, "gc_collected": None, "cuda_empty_cache": False}
        try:
            model = None
            result = None
            results = None
            cleanup["gc_collected"] = gc.collect()
            if torch_module is not None and torch_module.cuda.is_available():
                torch_module.cuda.empty_cache()
                if hasattr(torch_module.cuda, "ipc_collect"):
                    try:
                        torch_module.cuda.ipc_collect()
                        cleanup["cuda_ipc_collect"] = True
                    except Exception as exc:
                        cleanup["cuda_ipc_collect"] = False
                        cleanup["cuda_ipc_collect_error"] = str(exc)
                cleanup["cuda_empty_cache"] = True
                cleanup["cuda_memory_allocated_after"] = int(torch_module.cuda.memory_allocated(0))
                cleanup["cuda_memory_reserved_after"] = int(torch_module.cuda.memory_reserved(0))
        except Exception as exc:
            cleanup["error"] = str(exc)
        emit_cleanup(cleanup)

    if result_payload is None:
        raise RuntimeError("Qwen3-ASR did not produce a result")
    result_payload["cleanup"] = cleanup
    return result_payload


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Transcribe audio with Qwen3-ASR for AutoSubs")
    parser.add_argument("audio_path")
    parser.add_argument("--language", default="auto")
    parser.add_argument("--context", default="")
    parser.add_argument("--device", default="auto")
    parser.add_argument("--max-new-tokens", type=int, default=4096)
    parser.add_argument("--max-batch-size", type=int, default=8)
    parser.add_argument("--no-alignment", dest="alignment", action="store_false")
    parser.set_defaults(alignment=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        result = transcribe(args)
    except Exception as exc:
        print(f"Qwen3-ASR sidecar failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
