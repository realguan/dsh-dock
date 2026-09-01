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


ARTIFACTS = {
    "DSH Dock.app.tar.gz": "darwin-aarch64",
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


if __name__ == "__main__":
    unittest.main()
