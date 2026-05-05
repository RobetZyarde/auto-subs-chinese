#!/usr/bin/env python3
"""Qwen3-ASR sidecar for AutoSubs.

The Rust app invokes this script as a subprocess and reads a single JSON object
from stdout. Diagnostics must go to stderr so stdout stays parseable.
"""

from __future__ import annotations

import argparse
import json
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


@dataclass
class TimestampItem:
    text: str
    start: float
    end: float


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
    import torch
    from qwen_asr import Qwen3ASRModel

    audio_path = str(Path(args.audio_path).resolve())
    qwen_language = normalize_language_for_qwen(args.language)
    device_map = args.device if args.device != "auto" else ("cuda:0" if torch.cuda.is_available() else "cpu")
    use_cuda = str(device_map).startswith("cuda")
    dtype = torch.bfloat16 if use_cuda else torch.float32

    model_kwargs: dict[str, Any] = {
        "dtype": dtype,
        "device_map": device_map,
        "max_inference_batch_size": args.max_batch_size,
        "max_new_tokens": args.max_new_tokens,
    }
    if args.alignment:
        model_kwargs["forced_aligner"] = ALIGNER_MODEL
        model_kwargs["forced_aligner_kwargs"] = {"dtype": dtype, "device_map": device_map}

    model = Qwen3ASRModel.from_pretrained(ASR_MODEL, **model_kwargs)
    results = model.transcribe(
        audio=audio_path,
        context=args.context or None,
        language=qwen_language,
        return_time_stamps=args.alignment,
    )
    result = results[0] if isinstance(results, (list, tuple)) else results

    result_text = str(get_field(result, "text") or "")
    result_language = get_field(result, "language")
    timestamps = extract_timestamps(get_field(result, "time_stamps", "timestamps"))
    timestamps, result_text = merge_transcript_text(result_text, timestamps)
    duration = audio_duration_seconds(audio_path)
    segments = build_segments(result_text, timestamps, qwen_language, duration)

    return {
        "segments": segments,
        "language": normalize_language_for_autosubs(result_language, args.language),
        "model": ASR_MODEL,
    }


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
