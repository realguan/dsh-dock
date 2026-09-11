"""发布平台矩阵一致性守卫：`fetch-pnpm-bundle.sh` 白名单 ⊇ 壳能请求的平台集合。

## 为什么需要这条闸门（2026-09-11，G3）

`fetch-pnpm-bundle.sh` 的平台白名单曾漏 `win32-arm64`，而
`updates.rs::engine_pnpm_bundle()` 对 windows+aarch64 **会映射出** `win32-arm64`。
两者不一致 ⇒ 真到构建 Windows ARM64 包时，脚本会直接
`echo "不支持的平台名" ; return 2`，后果是**内置 pnpm 从未随包发出**，
直到运行时引擎引导才暴露（失败虽响亮——`tar -xzf` 非零退出——但发现得太晚）。

**「映射集合」与「打包支持集合」必须相等**是一条**不变式**，不靠人记得：
本文件用机器守住它，任何人加平台时都会立刻看到 rm 侧是否也齐。
（同族教训：R1「交叉/目标平台必须实测」、以及 G3 本身——本机绿灯不等于 CI 绿。）

## 三个被比对的集合

1. `fetch_one` 白名单 —— 脚本**能取**的平台（落位名 `<platform>.tgz`）
2. `uname` 自动探测分支 —— 无参调用时脚本**会选**的落位名
   （必须都落在白名单内，否则本机跑无参调用直接 `exit 2`）
3. `engine_pnpm_bundle()` 可产出集合 —— 壳**会请求**的落位名
4. CI matrix 的 `pnpm_platform` —— 各 leg **实际会取**的落位名

判据：3 ⊆ 1 且 4 ⊆ 1 且 2 ⊆ 1。**任一条不成立即红。**
"""

from __future__ import annotations

import importlib.util
import re
import unittest
from pathlib import Path


_REPO = Path(__file__).parents[2]
_FETCH_SH = _REPO / "scripts" / "fetch-pnpm-bundle.sh"
_UPDATES_RS = _REPO / "src-tauri" / "src" / "updates.rs"
_WORKFLOW = _REPO / ".github" / "workflows" / "build.yml"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def fetch_whitelist() -> set[str]:
    """`fetch_one` 的 case 白名单（脚本「能取」的平台）。"""
    text = _read(_FETCH_SH)
    # 定位 fetch_one 函数体内的 case 分支，避免误匹配 uname 探测块。
    body = text.split("fetch_one()", 1)[1]
    arm = re.search(r"case \"\$PLATFORM\" in\s*\n\s*([^)\n]+)\)", body)
    assert arm is not None, "未能从 fetch-pnpm-bundle.sh 解析平台白名单"
    return set(arm.group(1).split("|"))


def uname_detection_platforms() -> set[str]:
    """无参调用时 `uname` 分支会选出的落位名（`set -- <name>`）。"""
    text = _read(_FETCH_SH)
    block = text.split("if [[ $# -eq 0 ]]", 1)[1].split("fi", 1)[0]
    return set(re.findall(r"set -- ([a-z0-9-]+)", block))


def engine_bundle_platforms() -> set[str]:
    """`engine_pnpm_bundle()` 可能返回的 `<platform>.tgz` 平台名。

    以「函数体内出现的全部平台字面量」为准：新增分支若用了未登记的名字，
    本测试会把它算进来并要求白名单覆盖——这正是想要的严格性。
    """
    text = _read(_UPDATES_RS)
    body = text.split("pub fn engine_pnpm_bundle", 1)[1].split("\n}\n", 1)[0]
    # 排除 node 发行包命名（win-arm64/win-x64）等无关字符串：只取带平台前缀的形态。
    found = set(re.findall(r'"((?:darwin|linux|win32)-[a-z0-9]+)"', body))
    assert found, "未能从 engine_pnpm_bundle 解析出平台名"
    return found


def ci_matrix_pnpm_platforms() -> set[str]:
    """CI matrix 中各 leg 显式声明的 `pnpm_platform`（空串 = 走无参调用，不计）。"""
    text = _read(_WORKFLOW)
    raw = re.findall(r"^\s*pnpm_platform:\s*(\S*)\s*$", text, re.M)
    # YAML 里空值写作 `""`（裸 `""` 也是合法 YAML）⇒ 去引号后再判空。
    return {v.strip().strip("\"'") for v in raw if v.strip().strip("\"'")}


class ReleasePlatformMatrixTests(unittest.TestCase):
    """守住「壳会请求的账号」⊆「脚本能取的账号」。"""

    def test_fetch_script_accepts_every_platform_the_shell_can_request(self) -> None:
        """**核心不变式**：engine_pnpm_bundle 能映射出的平台，脚本必须都能取。

        这条若红，说明有人加了平台映射却没同步打包脚本 —— 症状是
        「内置 pnpm 从未随包发出」，只在运行时才炸。
        """
        whitelist = fetch_whitelist()
        requested = engine_bundle_platforms()
        missing = requested - whitelist
        self.assertFalse(
            missing,
            f"engine_pnpm_bundle 会请求但脚本拒绝的平台：{sorted(missing)}；"
            f"白名单现状 = {sorted(whitelist)}",
        )

    def test_uname_detection_stays_within_whitelist(self) -> None:
        """无参调用分支选出的落位名必须都在白名单内（否则本机无参调用直接退出）。"""
        whitelist = fetch_whitelist()
        detected = uname_detection_platforms()
        self.assertTrue(detected, "未解析到 uname 探测分支")
        self.assertFalse(
            detected - whitelist,
            f"uname 探测会选出但白名单不含：{sorted(detected - whitelist)}",
        )

    def test_ci_matrix_platforms_are_fetchable(self) -> None:
        """CI 各 leg 显式指定的 pnpm 平台必须可被脚本取到。"""
        whitelist = fetch_whitelist()
        declared = ci_matrix_pnpm_platforms()
        self.assertTrue(declared, "未从 workflow matrix 解析到 pnpm_platform")
        self.assertFalse(
            declared - whitelist,
            f"CI 声明但脚本取不到：{sorted(declared - whitelist)}",
        )

    def test_windows_arm64_is_registered_end_to_end(self) -> None:
        """G3 回归：`win32-arm64` 必须在白名单与 uname 探测两处都就位。

        2026-09-11 之前：`engine_pnpm_bundle` 已映射它，而脚本白名单与 uname 分支
        **都没有它** ⇒ 一旦构建 Windows ARM64 即失败。本用例钉住这条修复，
        **回退任一处即红**。
        """
        self.assertIn("win32-arm64", fetch_whitelist())
        self.assertIn("win32-arm64", uname_detection_platforms())
        self.assertIn("win32-arm64", engine_bundle_platforms())


if __name__ == "__main__":
    unittest.main()
