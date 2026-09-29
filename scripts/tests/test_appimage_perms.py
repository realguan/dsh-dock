"""`scripts/check-appimage-perms.py` 的回归测试 + CI 接线守卫。

## 为什么这两类断言必须同时存在（2026-09-28）

- **解析逻辑**：闸门读的是 `unsquashfs -lln` 的文本输出，而它只出现在 Linux
  （本仓开发机为 macOS，无 squashfs 工具）⇒ 解析规则只能靠内联 fixture 回归，
  否则"格式微调把判据改哑"这种失效没有任何红灯。
- **CI 接线**：`AppRun.wrapped` 的 0o770 只在「root 挂载 / 他人 uid 运行」时暴露，
  构建者本机与 CI 内自测都无感（tauri-apps/tauri#16155）。因此闸门一旦被删、
  被挪到 build 之前、或被加上 tag 限定（PR 构建不跑），缺陷会**静默复活**，
  直到目录站再次拒收才发现 —— 这类"删一行就静默失效"必须有机器守卫。

判据：预置步骤（`apprun-old` + `chmod 755`）必须 Linux-only、必须在**第一次
`cargo tauri build` 之前**、且不得只对 tag 生效；权限闸门步骤必须 Linux-only、
必须调用本脚本、必须在 build 之后；`squashfs-tools` 必须在系统依赖里。
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import unittest

_REPO = Path(__file__).parents[2]
_SCRIPT_PATH = Path(__file__).parents[1] / "check-appimage-perms.py"
_WORKFLOW = _REPO / ".github" / "workflows" / "build.yml"

_SPEC = importlib.util.spec_from_file_location("check_appimage_perms", _SCRIPT_PATH)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
# 脚本用了 `from __future__ import annotations` + dataclass ⇒ 3.9/3.10 上
# dataclasses 会回查 `sys.modules[cls.__module__]`，必须先注册再 exec。
sys.modules[_SPEC.name] = _MODULE
_SPEC.loader.exec_module(_MODULE)

find_violations = _MODULE.find_violations
mode_bits = _MODULE.mode_bits
parse_listing = _MODULE.parse_listing
main = _MODULE.main


# v1.3.3 线上产物的真实形态（头 6 行；此处刻意并入 0o770 那一行）。
_CLEAN_LISTING = """\
Parallel unsquashfs: Using 4 processors
drwxr-xr-x 0/0 105 2026-09-24 01:31 squashfs-root
-rwxr-xr-x 0/0 274 2026-09-24 01:31 squashfs-root/AppRun
lrwxrwxrwx 0/0 12 2026-09-24 01:31 squashfs-root/.DirIcon -> DSH Dock.png
-rw-r--r-- 0/0 21353 2026-09-24 01:31 squashfs-root/DSH Dock.png
drwxr-xr-x 0/0 46 2026-09-24 01:31 squashfs-root/apprun-hooks
-rwxr-xr-x 0/0 10070768 2026-09-24 01:31 squashfs-root/usr/bin/dsh-dock
"""

_DEFECT_LINE = "-rwxrwx--- 0/0 31552 2026-09-24 01:31 squashfs-root/AppRun.wrapped"


class ModeBitsTest(unittest.TestCase):
    def test_standard_modes(self) -> None:
        self.assertEqual(mode_bits("rwxr-xr-x"), 0o755)
        self.assertEqual(mode_bits("rwxrwx---"), 0o770)
        self.assertEqual(mode_bits("rw-r--r--"), 0o644)
        self.assertEqual(mode_bits("rwx------"), 0o700)

    def test_setuid_setgid_sticky(self) -> None:
        # s/t 带 x 位 ⇒ 按可执行算；`S` 无 x 位 ⇒ 特殊位仍在、可执行位不在。
        self.assertEqual(mode_bits("rwsr-sr-x"), 0o6755)
        self.assertEqual(mode_bits("rwSr--r--"), 0o4644)
        self.assertEqual(mode_bits("rwxrwxrwt"), 0o1777)


class ParseListingTest(unittest.TestCase):
    def test_clean_listing_parses_without_violations(self) -> None:
        entries = parse_listing(_CLEAN_LISTING)
        self.assertEqual(len(entries), 6)  # 处理器的提示行不算条目
        self.assertEqual(find_violations(entries), [])

    def test_defect_line_is_the_only_violation(self) -> None:
        entries = parse_listing(_CLEAN_LISTING + _DEFECT_LINE + "\n")
        violations = find_violations(entries)
        self.assertEqual(len(violations), 1)
        entry, reason = violations[0]
        self.assertEqual(entry.path, "squashfs-root/AppRun.wrapped")
        self.assertEqual(entry.mode, "-rwxrwx---")
        self.assertIn("other 不可执行", reason)

    def test_directory_without_other_traverse_is_reported(self) -> None:
        listing = "drwxr-x--- 0/0 46 2026-09-24 01:31 squashfs-root/apprun-hooks\n"
        violations = find_violations(parse_listing(listing))
        self.assertEqual([v[0].path for v in violations], ["squashfs-root/apprun-hooks"])
        self.assertIn("不可进入", violations[0][1])

    def test_file_without_other_read_is_reported(self) -> None:
        listing = "-rw-r----- 0/0 12 2026-09-24 01:31 squashfs-root/secret.txt\n"
        violations = find_violations(parse_listing(listing))
        self.assertEqual([v[0].path for v in violations], ["squashfs-root/secret.txt"])
        self.assertIn("不可读", violations[0][1])

    def test_symlink_is_exempt(self) -> None:
        entries = parse_listing("lrwxrwxrwx 0/0 12 2026-09-24 01:31 squashfs-root/x -> y\n")
        self.assertEqual(entries[0].path, "squashfs-root/x")  # target 不并入路径
        self.assertEqual(find_violations(entries), [])

    def test_name_with_spaces_is_kept_intact(self) -> None:
        entries = parse_listing(_CLEAN_LISTING)
        self.assertIn("squashfs-root/DSH Dock.png", [e.path for e in entries])

    def test_setuid_without_other_exec_is_not_a_violation(self) -> None:
        # `S` 表示"有 setuid 但无 x"⇒ 不是可执行文件，不该命中原判据。
        listing = "-rwSr--r-- 0/0 12 2026-09-24 01:31 squashfs-root/weird\n"
        self.assertEqual(find_violations(parse_listing(listing)), [])

    def test_noise_lines_are_ignored(self) -> None:
        noise = "Parallel unsquashfs: Using 4 processors\n\ncreated 300 files\n"
        self.assertEqual(parse_listing(noise), [])


class CliTest(unittest.TestCase):
    def _run(self, text: str | None) -> tuple[int, str, str]:
        with tempfile.TemporaryDirectory() as tmp:
            if text is None:
                path = Path(tmp) / "listing.txt"  # 不创建 ⇒ 走"文件不存在"分支
            else:
                path = Path(tmp) / "listing.txt"
                path.write_text(text, encoding="utf-8")
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = main(["check-appimage-perms.py", str(path)])
        return code, out.getvalue(), err.getvalue()

    def test_clean_listing_exits_zero(self) -> None:
        code, out, _ = self._run(_CLEAN_LISTING)
        self.assertEqual(code, 0)
        self.assertIn("闸门通过：6 条", out)

    def test_defect_listing_exits_one_and_names_the_file(self) -> None:
        code, out, err = self._run(_CLEAN_LISTING + _DEFECT_LINE + "\n")
        self.assertEqual(code, 1)
        self.assertIn("AppRun.wrapped", out)
        self.assertIn("1/7 条违规", err)

    def test_empty_listing_is_never_green(self) -> None:
        code, _, err = self._run("Parallel unsquashfs: Using 4 processors\n")
        self.assertEqual(code, 1)
        self.assertIn("拒绝静默通过", err)

    def test_missing_listing_file_exits_two(self) -> None:
        code, _, err = self._run(None)
        self.assertEqual(code, 2)
        self.assertIn("listing 文件不存在", err)

    def test_usage_without_arguments_exits_two(self) -> None:
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = main(["check-appimage-perms.py"])
        self.assertEqual(code, 2)


class WorkflowWiringTest(unittest.TestCase):
    """闸门与绕行步骤的接线守卫：删掉/挪位/加 tag 限定都必须变红。"""

    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = _WORKFLOW.read_text(encoding="utf-8")

    def _step(self, marker: str) -> str:
        blocks = [b for b in self.workflow.split("\n      - name: ") if marker in b]
        self.assertEqual(len(blocks), 1, f"工作流中匹配 {marker!r} 的步骤应恰好 1 个")
        return blocks[0]

    def test_squashfs_tools_is_installed(self) -> None:
        self.assertIn("squashfs-tools", self.workflow)

    def test_apprun_preseed_runs_on_linux_before_any_build(self) -> None:
        step = self._step("apprun-old")
        self.assertIn("runner.os == 'Linux'", step)
        self.assertIn("chmod 755", step)
        # PR 构建也必须绕行（只对 tag 生效 = PR 出的包带 0o770，问题只是换个地方暴露）
        self.assertNotIn("github.ref_type", step)
        self.assertLess(
            self.workflow.index("apprun-old"),
            self.workflow.index("cargo tauri build"),
            "预置步骤必须早于第一次 cargo tauri build",
        )

    def test_permission_gate_exists_after_build(self) -> None:
        step = self._step("check-appimage-perms.py")
        self.assertIn("runner.os == 'Linux'", step)
        self.assertIn("unsquashfs", step)
        self.assertIn("--appimage-offset", step)
        self.assertNotIn("continue-on-error", step)  # 闸门不许"永不红"
        self.assertGreater(
            self.workflow.index("check-appimage-perms.py"),
            self.workflow.index("cargo tauri build"),
            "权限闸门必须晚于第一次 cargo tauri build（要审的是产物）",
        )


if __name__ == "__main__":
    unittest.main()
