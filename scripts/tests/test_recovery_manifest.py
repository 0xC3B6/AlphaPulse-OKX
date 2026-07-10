import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scripts.recovery_manifest import build_manifest


class RecoveryManifestTest(unittest.TestCase):
    def test_keeps_only_successful_old_worktree_patch_events(self):
        records = [
            {
                "timestamp": "2026-07-01T00:00:00Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "apply_patch",
                    "call_id": "call_ok",
                    "input": "*** Begin Patch\n*** Add File: /repo/.worktrees/radar-pattern-signals/backend/src/auto_strategy.rs\n+ok\n*** End Patch",
                },
            },
            {
                "timestamp": "2026-07-01T00:00:01Z",
                "type": "event_msg",
                "payload": {
                    "type": "patch_apply_end",
                    "call_id": "call_ok",
                    "success": True,
                    "changes": {
                        "/repo/.worktrees/radar-pattern-signals/backend/src/auto_strategy.rs": {
                            "type": "add",
                            "unified_diff": "",
                        }
                    },
                },
            },
            {
                "timestamp": "2026-07-01T00:00:02Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "apply_patch",
                    "call_id": "call_failed",
                    "input": "*** Begin Patch\n*** Delete File: /repo/.worktrees/radar-pattern-signals/backend/src/runtime.rs\n*** End Patch",
                },
            },
            {
                "timestamp": "2026-07-01T00:00:03Z",
                "type": "event_msg",
                "payload": {
                    "type": "patch_apply_end",
                    "call_id": "call_failed",
                    "success": False,
                    "changes": {
                        "/repo/.worktrees/radar-pattern-signals/backend/src/runtime.rs": {
                            "type": "delete",
                            "unified_diff": "-failed\n",
                        }
                    },
                },
            },
        ]
        with tempfile.TemporaryDirectory() as directory:
            session = Path(directory) / "session.jsonl"
            session.write_text("".join(json.dumps(row) + "\n" for row in records))
            digest = hashlib.sha256(session.read_bytes()).hexdigest()
            manifest = build_manifest(session, digest)
            with self.assertRaisesRegex(ValueError, "session hash mismatch"):
                build_manifest(session, "0" * 64)

        self.assertEqual(manifest["base_commit"], "907b9ecac5acf0b1f78ce9cf0c5a3be0c3cd0285")
        self.assertEqual(manifest["session_sha256"], digest)
        self.assertEqual(len(manifest["patches"]), 1)
        self.assertEqual(manifest["successful_patch_event_count"], 1)
        self.assertEqual(manifest["changed_file_count"], 1)
        self.assertEqual(manifest["patches"][0]["call_id"], "call_ok")
        self.assertNotIn("call_failed", {patch["call_id"] for patch in manifest["patches"]})
        self.assertEqual(manifest["patches"][0]["path"], "backend/src/auto_strategy.rs")
        self.assertEqual(manifest["patches"][0]["change_type"], "add")
        self.assertEqual(
            manifest["patches"][0]["patch_sha256"],
            hashlib.sha256(records[0]["payload"]["input"].encode()).hexdigest(),
        )
        self.assertEqual(
            manifest["patches"][0]["diff_sha256"],
            hashlib.sha256(b"").hexdigest(),
        )
        self.assertFalse(any(
            component["classification"] == "behavior_only"
            for component in manifest["components"]
        ))

    def test_rejects_successful_old_worktree_event_without_matching_call(self):
        records = [
            {
                "timestamp": "2026-07-01T00:00:00Z",
                "type": "event_msg",
                "payload": {
                    "type": "patch_apply_end",
                    "call_id": "call_missing",
                    "success": True,
                    "changes": {
                        "/repo/.worktrees/radar-pattern-signals/backend/src/runtime.rs": {
                            "type": "update",
                            "unified_diff": "+missing\n",
                        }
                    },
                },
            },
        ]
        with tempfile.TemporaryDirectory() as directory:
            session = Path(directory) / "session.jsonl"
            session.write_text(
                "".join(json.dumps(row) + "\n" for row in records),
                encoding="utf-8",
                newline="\n",
            )
            digest = hashlib.sha256(session.read_bytes()).hexdigest()
            with self.assertRaisesRegex(
                ValueError,
                "missing apply_patch call for successful patch event: call_missing",
            ):
                build_manifest(session, digest)

    def test_rejects_duplicate_apply_patch_call_id(self):
        records = [
            {
                "timestamp": "2026-07-01T00:00:00Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "apply_patch",
                    "call_id": "call_duplicate",
                    "input": "*** Begin Patch\n*** End Patch",
                },
            },
            {
                "timestamp": "2026-07-01T00:00:01Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "apply_patch",
                    "call_id": "call_duplicate",
                    "input": "*** Begin Patch\n*** Add File: duplicate\n*** End Patch",
                },
            },
            {
                "timestamp": "2026-07-01T00:00:02Z",
                "type": "event_msg",
                "payload": {
                    "type": "patch_apply_end",
                    "call_id": "call_duplicate",
                    "success": True,
                    "changes": {
                        "/repo/.worktrees/radar-pattern-signals/backend/src/runtime.rs": {
                            "type": "update",
                            "unified_diff": "+duplicate\n",
                        }
                    },
                },
            },
        ]
        with tempfile.TemporaryDirectory() as directory:
            session = Path(directory) / "session.jsonl"
            session.write_text(
                "".join(json.dumps(row) + "\n" for row in records),
                encoding="utf-8",
                newline="\n",
            )
            digest = hashlib.sha256(session.read_bytes()).hexdigest()
            with self.assertRaisesRegex(
                ValueError,
                "duplicate apply_patch call_id: call_duplicate",
            ):
                build_manifest(session, digest)

    def test_rejects_invalid_apply_patch_call_metadata(self):
        cases = [
            (
                {"call_id": "", "input": "*** Begin Patch\n*** End Patch"},
                "apply_patch call_id must be a non-empty string",
            ),
            (
                {"call_id": "call_bad_input", "input": None},
                "apply_patch input must be a string: call_bad_input",
            ),
        ]
        for call_fields, expected_error in cases:
            with self.subTest(expected_error=expected_error):
                records = [
                    {
                        "timestamp": "2026-07-01T00:00:00Z",
                        "type": "response_item",
                        "payload": {
                            "type": "custom_tool_call",
                            "name": "apply_patch",
                            **call_fields,
                        },
                    },
                ]
                with tempfile.TemporaryDirectory() as directory:
                    session = Path(directory) / "session.jsonl"
                    session.write_text(
                        "".join(json.dumps(row) + "\n" for row in records),
                        encoding="utf-8",
                        newline="\n",
                    )
                    digest = hashlib.sha256(session.read_bytes()).hexdigest()
                    with self.assertRaisesRegex(ValueError, expected_error):
                        build_manifest(session, digest)


if __name__ == "__main__":
    unittest.main()
