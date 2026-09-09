"""extract-release-notes.py 的单元回归测试。"""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

_SCRIPT_PATH = Path(__file__).parents[1] / "extract-release-notes.py"
_SPEC = importlib.util.spec_from_file_location("extract_release_notes", _SCRIPT_PATH)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)

extract_from_release_notes = _MODULE.extract_from_release_notes
extract_from_broadcasts = _MODULE.extract_from_broadcasts


SAMPLE_RELEASE_NOTES = """# Release Notes Header

## [v0.9.10] - 2026-09-05

### 🌟 核心亮点 (Highlights)
- 0.9.10 特性

## [v0.9.1] - 2026-09-01

### 🌟 核心亮点 (Highlights)
- 0.9.1 特性

## [0.9.0] - 2026-08-31

### 🌟 核心亮点 (Highlights)
- 0.9.0 特性 (标题无 v 前缀)
"""

SAMPLE_BROADCASTS = """# Broadcasts Header

### 2026-09-05 发版通知 · v0.9.10 发布 —— test

- 变更：v0.9.10 广播变更说明。

### 2026-09-01 完成通知 · 某项优化 —— test

- 变更：无版本号完成通知。
"""


class ExtractReleaseNotesTests(unittest.TestCase):
    """验证 Release Notes 提取与精确匹配契约。"""

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.rn_file = Path(self.temp_dir.name) / "RELEASE_NOTES.md"
        self.rn_file.write_text(SAMPLE_RELEASE_NOTES, encoding="utf-8")
        self.bc_file = Path(self.temp_dir.name) / "broadcasts.md"
        self.bc_file.write_text(SAMPLE_BROADCASTS, encoding="utf-8")

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_extract_exact_version_with_v(self) -> None:
        notes = extract_from_release_notes("v0.9.1", str(self.rn_file))
        self.assertTrue(notes.startswith("## [v0.9.1] - 2026-09-01"))
        self.assertIn("0.9.1 特性", notes)
        self.assertNotIn("0.9.10 特性", notes)

    def test_extract_exact_version_without_v_in_tag(self) -> None:
        notes = extract_from_release_notes("0.9.1", str(self.rn_file))
        self.assertTrue(notes.startswith("## [v0.9.1] - 2026-09-01"))
        self.assertIn("0.9.1 特性", notes)

    def test_extract_header_without_v_prefix(self) -> None:
        notes = extract_from_release_notes("v0.9.0", str(self.rn_file))
        self.assertTrue(notes.startswith("## [0.9.0] - 2026-08-31"))
        self.assertIn("0.9.0 特性", notes)

    def test_avoids_substring_collision(self) -> None:
        """测试 0.9.1 不会被 0.9.10 误截取。"""
        notes_10 = extract_from_release_notes("v0.9.10", str(self.rn_file))
        self.assertTrue(notes_10.startswith("## [v0.9.10] - 2026-09-05"))

        notes_1 = extract_from_release_notes("v0.9.1", str(self.rn_file))
        self.assertTrue(notes_1.startswith("## [v0.9.1] - 2026-09-01"))

    def test_missing_version_returns_empty(self) -> None:
        notes = extract_from_release_notes("v0.9.99", str(self.rn_file))
        self.assertEqual(notes, "")

    def test_broadcast_fallback_exact_match(self) -> None:
        notes = extract_from_broadcasts("v0.9.10", str(self.bc_file))
        self.assertTrue(notes.startswith("## 2026-09-05 发版通知 · v0.9.10 发布 —— test"))
        self.assertIn("v0.9.10 广播变更说明", notes)


if __name__ == "__main__":
    unittest.main()
