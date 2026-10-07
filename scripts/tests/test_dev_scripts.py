"""dev / tdev 两条开发启动命令的「目录口径」闸门（2026-10-07）。

## 为什么需要这条闸门

`dev` 与 `tdev` 的**唯一区别**就是"用哪两个目录"，而这两个目录**都不是肉眼可见的**：
一个由 app identifier 决定（数据目录），一个由 `resolve.rs::user_dsh_home` 决定
（dsh home，判据 `cfg!(debug_assertions)` + 最高优先的 `DSH_HOME`）。写反了不会报错、
不会红、只会在几天后表现为"我的正式 profile 被开发构建改了"或"隔离档莫名写了真实数据"。

故把口径钉成机器判据（改动即红）：

| 命令 | 数据目录（identifier） | dsh home | 启动链 |
| :--- | :--- | :--- | :--- |
| `scripts/dev.sh` | `io.github.realguan.dsh-dock`（正式） | `$HOME/.dsh` | `cargo tauri dev` |
| `scripts/tdev.sh` | `io.github.realguan.dsh-dock.dev`（隔离） | `$HOME/.dsh-dock-dev` | `cargo tauri dev --config tauri.dev.conf.json` |

判据取自脚本自身的 `--print` 输出（不起 GUI、不起 vite、不 spawn 任何进程），
即断言的是**脚本真正会导出/执行的东西**，而不是源码文本。

## 运行

    PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v

（CI 在 Linux leg 与其余发布辅助测试同批执行。）
"""

import os
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"


def print_mode(script: str) -> dict:
    """跑 `<script> --print`，把 `k=v` 行解析成 dict（脚本保证不启动任何东西）。"""
    proc = subprocess.run(
        ["bash", str(SCRIPTS / script), "--print"],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        check=True,
    )
    out = {}
    for line in proc.stdout.splitlines():
        if "=" in line:
            key, _, value = line.partition("=")
            out[key.strip()] = value.strip()
    return out


def home() -> str:
    return os.environ.get("HOME") or os.path.expanduser("~")


class DevScriptsTest(unittest.TestCase):
    def test_dev_uses_production_dirs(self) -> None:
        kv = print_mode("dev.sh")
        self.assertEqual(kv["identifier"], "io.github.realguan.dsh-dock", "dev 必须是正式 identifier")
        self.assertEqual(kv["DSH_HOME"], os.path.join(home(), ".dsh"), "dev 必须用正式 ~/.dsh")
        self.assertEqual(kv["command"], "cargo tauri dev", "dev 不得带 --config（那会切到隔离 identifier）")

    def test_tdev_uses_isolated_dirs(self) -> None:
        kv = print_mode("tdev.sh")
        self.assertEqual(
            kv["identifier"], "io.github.realguan.dsh-dock.dev", "tdev 必须是隔离 identifier"
        )
        self.assertEqual(
            kv["DSH_HOME"], os.path.join(home(), ".dsh-dock-dev"), "tdev 必须用隔离 home"
        )
        self.assertEqual(
            kv["command"],
            "cargo tauri dev --config tauri.dev.conf.json",
            "tdev 必须带 dev 配置（identifier 隔离的唯一来源）",
        )

    def test_two_modes_never_share_a_home(self) -> None:
        """两条命令共用 home = 隔离失效（ADR-0015 §1.2 那条事故的放大器）。"""
        self.assertNotEqual(print_mode("dev.sh")["DSH_HOME"], print_mode("tdev.sh")["DSH_HOME"])
        self.assertNotEqual(print_mode("dev.sh")["identifier"], print_mode("tdev.sh")["identifier"])

    def test_scripts_are_executable(self) -> None:
        for name in ("dev.sh", "tdev.sh"):
            path = SCRIPTS / name
            self.assertTrue(path.is_file(), f"{path} 不存在")
            self.assertTrue(os.access(path, os.X_OK), f"{path} 缺可执行位（入库须 100755）")

    def test_scripts_pin_dsh_home_regardless_of_caller_env(self) -> None:
        """调用环境里已导出 DSH_HOME 时，两条命令仍须给出各自确定的 home。

        这是 tdev 的真陷阱：`DSH_HOME` 优先级最高（用户主权），环境里一旦有人
        导出过它（从 dsh 会话起的终端就是），debug 的隔离默认值会被静默绕过。
        """
        for script, expected in (("dev.sh", ".dsh"), ("tdev.sh", ".dsh-dock-dev")):
            proc = subprocess.run(
                ["bash", str(SCRIPTS / script), "--print"],
                cwd=str(ROOT),
                capture_output=True,
                text=True,
                check=True,
                env={**os.environ, "DSH_HOME": "/tmp/should-be-ignored"},
            )
            self.assertIn(
                f"DSH_HOME={os.path.join(home(), expected)}",
                proc.stdout,
                f"{script} 未钉住自己的 dsh home（被调用环境的 DSH_HOME 带偏）",
            )


if __name__ == "__main__":
    unittest.main()
