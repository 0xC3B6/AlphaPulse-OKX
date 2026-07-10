#!/usr/bin/env python3
import argparse
import hashlib
import json
from pathlib import Path

BASE_COMMIT = "907b9ecac5acf0b1f78ce9cf0c5a3be0c3cd0285"
WORKTREE_MARKER = "/.worktrees/radar-pattern-signals/"
RECOVERY_COMPONENTS = [
    {"path": "backend/src/indicators/patterns.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/src/indicators/scalping.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/src/time_regime.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/src/market_context.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/src/auto_strategy.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/tests/auto_strategy.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/tests/state_prices.rs", "classification": "exact_replay", "source": "session"},
    {"path": "backend/src/domain.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/indicators/mod.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/scoring.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/runtime.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/paper.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/state.rs", "classification": "verified_port", "source": "session+907b9ec"},
    {"path": "backend/src/persistence.rs", "classification": "verified_port", "source": "7be9e81"},
]


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _iter_records(path: Path, required_token: bytes):
    with path.open("rb") as stream:
        for line in stream:
            if required_token in line:
                yield json.loads(line)


def build_manifest(session_path: Path, expected_sha256: str) -> dict:
    actual_sha256 = _sha256_file(session_path)
    if actual_sha256 != expected_sha256:
        raise ValueError(f"session hash mismatch: {actual_sha256}")

    calls = {}
    for record in _iter_records(session_path, b'"apply_patch"'):
        payload = record.get("payload", {})
        if (
            record.get("type") == "response_item"
            and payload.get("type") in {"custom_tool_call", "function_call"}
            and payload.get("name") == "apply_patch"
        ):
            call_id = payload.get("call_id")
            if not isinstance(call_id, str) or not call_id:
                raise ValueError("apply_patch call_id must be a non-empty string")
            patch_text = payload.get("input")
            if not isinstance(patch_text, str):
                raise ValueError(f"apply_patch input must be a string: {call_id}")
            if call_id in calls:
                raise ValueError(f"duplicate apply_patch call_id: {call_id}")
            calls[call_id] = sha256_bytes(patch_text.encode())

    patches = []
    for record in _iter_records(session_path, b'"patch_apply_end"'):
        payload = record.get("payload", {})
        if not (
            record.get("type") == "event_msg"
            and payload.get("type") == "patch_apply_end"
            and payload.get("success") is True
        ):
            continue
        call_id = payload.get("call_id")
        changes = [
            (absolute_path, change)
            for absolute_path, change in sorted(payload.get("changes", {}).items())
            if WORKTREE_MARKER in absolute_path
        ]
        if not changes:
            continue
        if not isinstance(call_id, str) or not call_id:
            raise ValueError("successful patch event call_id must be a non-empty string")
        if call_id not in calls:
            raise ValueError(
                f"missing apply_patch call for successful patch event: {call_id}"
            )
        patch_sha256 = calls[call_id]
        for absolute_path, change in changes:
            patches.append(
                {
                    "timestamp": record.get("timestamp"),
                    "call_id": call_id,
                    "path": absolute_path.split(WORKTREE_MARKER, 1)[1],
                    "change_type": change.get("type"),
                    "patch_sha256": patch_sha256,
                    "diff_sha256": sha256_bytes(change.get("unified_diff", "").encode()),
                }
            )

    patches.sort(key=lambda item: (item["timestamp"], item["path"], item["call_id"]))
    return {
        "base_commit": BASE_COMMIT,
        "session_path": str(session_path),
        "session_sha256": actual_sha256,
        "successful_patch_event_count": len({patch["call_id"] for patch in patches}),
        "changed_file_count": len(patches),
        "components": RECOVERY_COMPONENTS,
        "patches": patches,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--session", type=Path, required=True)
    parser.add_argument("--expected-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    manifest = build_manifest(args.session, args.expected_sha256)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


if __name__ == "__main__":
    main()
