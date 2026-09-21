"""`render-product.sh` 的快照档落位名闸门（平台审计 A6，2026-09-21）。

## 为什么需要它

脚本此前把传入的 node 可执行文件**一律**落成 `dsh-node`（无扩展名）。Windows 的 node
叫 `node.exe`，于是：

- `docs/contract.md` 明写三平台各自注入自己的 node（`node` / `node.exe`），脚本只做了一平台；
- 壳侧 `shell.rs` 先 `is_file()` 判存在，再经 `child_cmd` 走 CreateProcess —— 无扩展名时
  系统会**自动追加 `.exe`**，于是找不到那个文件 ⇒ **Windows 快照档启动必败**，错误卡见用户；
- 该脚本**全仓 CI 零引用**（装配方/打包期用），所以没有任何闸门会发现它。

## 判据

真跑脚本两遍（`--node` 分别传 `node` 与 `node.exe`），断言：

1. 落位名与传入平台一致，且**另一个名字不存在**（反例方向）；
2. `product.manifest.json` 的 `snapshot.nodeBin` 与磁盘上的真实文件名**逐字一致**
   （manifest 是壳解析的入口，两处漂移即等于没改）；
3. `--out` 被真正尊重 —— 这条同时守护"测试不许覆写仓库本体"：脚本若忽略 `--out`，
   仓库的 `src-tauri/resources/product.manifest.json` 会被写脏，本用例立即红。
"""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

_REPO = Path(__file__).parents[2]
_SCRIPT = _REPO / "scripts" / "render-product.sh"
_REPO_MANIFEST = _REPO / "src-tauri" / "resources" / "product.manifest.json"


class RenderProductNodeNameTests(unittest.TestCase):
    """三平台 node 落位名 + manifest 单源 + `--out` 生效。"""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="render-product-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

        # 假运行时树：只需满足脚本的三个前置检查
        self.runtime = self.tmp / "dsh-runtime"
        (self.runtime / "@deepseek-ai" / "dsh" / "lib").mkdir(parents=True)
        (self.runtime / "@deepseek-ai" / "dsh" / "lib" / "bin.js").write_text("// dsh\n", encoding="utf-8")

        self.home = self.tmp / "dsh-home"
        (self.home / "profiles" / "web").mkdir(parents=True)

        # 假 src-tauri：脚本会改写它的 tauri.conf.json（真跑，故必须给临时目录）
        self.out = self.tmp / "src-tauri"
        (self.out / "resources").mkdir(parents=True)
        (self.out / "tauri.conf.json").write_text(
            json.dumps({"productName": "x", "identifier": "x", "app": {"windows": []}}),
            encoding="utf-8",
        )

        # 仓库本体在测试前后的指纹（守护 `--out` 被忽略的情形）
        self.repo_manifest_before = (
            _REPO_MANIFEST.read_bytes() if _REPO_MANIFEST.is_file() else None
        )

    def run_script(self, node_name: str) -> dict:
        node = self.tmp / node_name
        node.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        node.chmod(0o755)

        proc = subprocess.run(
            [
                "bash",
                str(_SCRIPT),
                "--node", str(node),
                "--dsh-runtime", str(self.runtime),
                "--dsh-home", str(self.home),
                "--profile", "web",
                "--name", "DSH Dock",
                "--id", "dev.deepseek.dsh-dock",
                "--out", str(self.out),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc.returncode, 0, f"脚本执行失败：{proc.stderr}")
        manifest = json.loads(
            (self.out / "resources" / "product.manifest.json").read_text(encoding="utf-8")
        )
        return manifest

    def test_windows_node_keeps_the_exe_suffix(self) -> None:
        """`node.exe` 进 ⇒ `dsh-node.exe` 出，且 manifest 指同一个名字。"""
        manifest = self.run_script("node.exe")

        node_bin = manifest["snapshot"]["nodeBin"]
        self.assertEqual(
            node_bin,
            "dsh-snapshot/node/bin/dsh-node.exe",
            "Windows 的 node 必须保留 .exe：无扩展名时 CreateProcess 自动追加 .exe 会找不到文件",
        )
        landed = self.out / "resources" / node_bin
        self.assertTrue(landed.is_file(), f"manifest 指的 {node_bin} 在磁盘上不存在")
        self.assertFalse(
            (self.out / "resources" / "dsh-snapshot/node/bin/dsh-node").exists(),
            "不得同时留下无扩展名的残影（会把『找不到』伪装成『找得到』）",
        )

    def test_unix_node_stays_extensionless(self) -> None:
        """`node` 进 ⇒ `dsh-node` 出（macOS / Linux 不得被加上 .exe）。"""
        manifest = self.run_script("node")

        node_bin = manifest["snapshot"]["nodeBin"]
        self.assertEqual(node_bin, "dsh-snapshot/node/bin/dsh-node")
        self.assertTrue((self.out / "resources" / node_bin).is_file())
        self.assertFalse(
            (self.out / "resources" / "dsh-snapshot/node/bin/dsh-node.exe").exists(),
            "非 Windows 不得落 .exe",
        )

    def test_out_flag_never_touches_the_repository(self) -> None:
        """`--out` 必须真的生效 —— 否则真跑会把仓库本体的 manifest 写脏。"""
        self.run_script("node")

        # 本次临时 out 目录拿到了文件
        self.assertTrue((self.out / "resources" / "product.manifest.json").is_file())
        # 仓库本体指纹未变
        after = _REPO_MANIFEST.read_bytes() if _REPO_MANIFEST.is_file() else None
        self.assertEqual(
            self.repo_manifest_before,
            after,
            "--out 被忽略：脚本写到了仓库本体的 src-tauri/resources/product.manifest.json",
        )


if __name__ == "__main__":
    unittest.main()
