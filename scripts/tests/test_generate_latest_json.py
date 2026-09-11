"""generate-latest-json.py 的纯逻辑回归测试。"""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

_SCRIPT_PATH = Path(__file__).parents[1] / "generate-latest-json.py"
_SPEC = importlib.util.spec_from_file_location("generate_latest_json", _SCRIPT_PATH)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)

REQUIRED_TARGETS = _MODULE.REQUIRED_TARGETS
build_platforms = _MODULE.build_platforms
release_asset_name = _MODULE.release_asset_name
target_for_artifact = _MODULE.target_for_artifact


# 资产名 → updater 目标键。macOS 两份（arm64 / Intel）**必须带架构标记**：
# Tauri 默认名 `DSH Dock.app.tar.gz` 不含架构，两个 leg 会撞名，故 CI 按架构重命名。
ARTIFACTS = {
    "DSH Dock_aarch64.app.tar.gz": "darwin-aarch64",
    "DSH Dock_x86_64.app.tar.gz": "darwin-x86_64",
    "DSH Dock_0.9.3_amd64.AppImage": "linux-x86_64-appimage",
    "DSH Dock_0.9.3_amd64.deb": "linux-x86_64-deb",
    "DSH Dock-0.9.3-1.x86_64.rpm": "linux-x86_64-rpm",
    "DSH Dock_0.9.3_x64_en-US.msi": "windows-x86_64-msi",
    "DSH Dock_0.9.3_x64-setup.exe": "windows-x86_64-nsis",
}


class GenerateLatestJsonTests(unittest.TestCase):
    """验证发布资产到 updater feed 的映射契约。"""

    def test_builds_all_current_release_targets(self) -> None:
        """当前三平台安装器均应得到可更新的目标条目。"""
        assets = {
            release_asset_name(name): f"https://example.invalid/{index}"
            for index, name in enumerate(ARTIFACTS, start=1)
        }
        signatures = {name: f"signature-{index}" for index, name in enumerate(ARTIFACTS)}

        platforms = build_platforms(
            assets, signatures, "realguan/dsh-dock", "v0.9.3"
        )

        self.assertEqual(set(platforms), REQUIRED_TARGETS)
        for artifact_name, target in ARTIFACTS.items():
            self.assertEqual(
                platforms[target]["url"],
                "https://github.com/realguan/dsh-dock/releases/download/v0.9.3/"
                + release_asset_name(artifact_name),
            )

    def test_rejects_missing_deb_or_rpm_update_assets(self) -> None:
        """不能再发布只有 AppImage 更新条目的 Linux 发行包。"""
        assets = {
            release_asset_name(name): f"https://example.invalid/{index}"
            for index, name in enumerate(ARTIFACTS, start=1)
        }
        signatures = {
            name: f"signature-{index}"
            for index, name in enumerate(ARTIFACTS)
            if not name.endswith((".deb", ".rpm"))
        }

        with self.assertRaisesRegex(ValueError, "linux-x86_64-deb"):
            build_platforms(assets, signatures, "realguan/dsh-dock", "v0.9.3")

    def test_rejects_signature_without_uploaded_release_asset(self) -> None:
        """草稿中缺少任一被签名安装器时，禁止生成公开 feed。"""
        assets = {
            release_asset_name(name): f"https://example.invalid/{index}"
            for index, name in enumerate(ARTIFACTS, start=1)
            if not name.endswith(".rpm")
        }
        signatures = {name: f"signature-{index}" for index, name in enumerate(ARTIFACTS)}

        with self.assertRaisesRegex(ValueError, "未上传到 Release"):
            build_platforms(assets, signatures, "realguan/dsh-dock", "v0.9.3")

    def test_darwin_arch_markers_map_to_distinct_targets(self) -> None:
        """架构标记 → 目标键：两个 macOS 架构必须落到不同键（否则 Intel 包被顶替）。"""
        cases = {
            "DSH Dock_aarch64.app.tar.gz": "darwin-aarch64",
            "DSH Dock_arm64.app.tar.gz": "darwin-aarch64",
            "DSH Dock_x86_64.app.tar.gz": "darwin-x86_64",
            "DSH Dock_x64.app.tar.gz": "darwin-x86_64",
            "DSH Dock_amd64.app.tar.gz": "darwin-x86_64",
            "DSH Dock_universal.app.tar.gz": "darwin-universal",
            # 大小写不敏感
            "DSH Dock_AARCH64.app.tar.gz": "darwin-aarch64",
        }
        for artifact_name, expected in cases.items():
            with self.subTest(artifact_name=artifact_name):
                self.assertEqual(target_for_artifact(artifact_name), expected)

    def test_untagged_darwin_artifact_is_not_silently_mapped(self) -> None:
        """**反例（防静默发错架构）**：无架构标记的 `.app.tar.gz` 不得回落成任一架构。

        Tauri 默认名就是 `DSH Dock.app.tar.gz`。若此处回落成 `darwin-aarch64`，
        一旦 CI 漏掉打标步骤，Intel 包会被当成 Apple Silicon 包装进 feed——用户装上
        直接起不来。故判为不可映射，由完整性检查响亮失败。
        """
        self.assertIsNone(target_for_artifact("DSH Dock.app.tar.gz"))

        # 端到端：只有未打标的 macOS 资产 ⇒ 必须报「缺少目标」而不是发布错误 feed。
        others = {
            name: target for name, target in ARTIFACTS.items() if not name.endswith(".app.tar.gz")
        }
        assets = {
            release_asset_name(name): f"https://example.invalid/{index}"
            for index, name in enumerate(["DSH Dock.app.tar.gz", *others], start=1)
        }
        signatures = {
            name: f"signature-{index}"
            for index, name in enumerate(["DSH Dock.app.tar.gz", *others])
        }
        with self.assertRaisesRegex(ValueError, "darwin-aarch64"):
            build_platforms(assets, signatures, "realguan/dsh-dock", "v0.9.3")

    def test_duplicate_darwin_tarball_name_is_rejected(self) -> None:
        """两个 macOS leg 若撞名（未打标），feed 生成必须红——不是静默取其一。"""
        signatures = {
            "DSH Dock.app.tar.gz": "sig-a",
            "DSH Dock_0.9.3_amd64.deb": "sig-b",
        }
        assets = {release_asset_name(n): f"https://example.invalid/{n}" for n in signatures}
        # 先落一个已映射的 darwin 目标，再让第二个同名/同目标的资产出现 ⇒ 目标重复。
        signatures["DSH Dock_aarch64.app.tar.gz"] = "sig-c"
        assets[release_asset_name("DSH Dock_aarch64.app.tar.gz")] = "https://example.invalid/c"
        with self.assertRaises(ValueError):
            build_platforms(assets, signatures, "realguan/dsh-dock", "v0.9.3")


if __name__ == "__main__":
    unittest.main()
