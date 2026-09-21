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

## 四个被比对的集合

1. `fetch_one` 白名单 —— 脚本**能取**的平台（落位名 `<platform>.tgz`）
2. `uname` 自动探测分支 —— 无参调用时脚本**会选**的落位名
   （必须都落在白名单内，否则本机跑无参调用直接 `exit 2`）
3. `engine_pnpm_bundle()` 可产出集合 —— 壳**会请求**的落位名
4. CI matrix 的 `pnpm_platform` —— 各 leg **实际会取**的落位名

判据：3 ⊆ 1 且 4 ⊆ 1 且 2 ⊆ 1。**任一条不成立即红。**

## 平台覆盖铁律（`AGENTS.md` §0 红线 3，2026-09-21 维护者裁定）

**macOS（arm64 / x86_64）· Windows（x64，含 WSL2 客体模式）· Linux（x64），只要技术
允许就必须都适配**；任何人不许擅自判定「这个平台先不做」。本文件把这条铁律投影成机器
判据，使「收窄平台」不能靠改文档悄悄完成：

- 每个运行世界必须**三处齐备** —— ① 构建矩阵里有 leg ② 该 leg 上传安装器 + updater
  ③ 发布 job 下载它们。删任一处即红（删上传 = 平台静默不出包，最阴的一种）。
- 「矩阵构建出的工件集合」必须**恰好等于**「发布侧下载的工件集合」：漏发布（建了不发）
  与凭空下载（发了没建）都红。
- 构建 job 内**不得**出现 `continue-on-error`（那等于留着一个永远绿的红灯平台）。

删 leg / 删上传 / 删下载 / 加 `continue-on-error` 都是**改平台契约**，
须维护者裁定 + ADR —— 不是把本文件改绿就行。
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path
from typing import NamedTuple


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


def job_block(name: str) -> str:
    """取 workflow 中某个 job 的正文（`jobs:` 下 2 空格缩进的顶层键之间）。"""
    body = _read(_WORKFLOW).split("\njobs:\n", 1)[1]
    parts = re.split(r"(?m)^  ([\w-]+):\s*$", body)
    jobs = dict(zip(parts[1::2], parts[2::2]))
    assert name in jobs, f"build.yml 里找不到 job `{name}`；现有 = {sorted(jobs)}"
    return jobs[name]


def matrix_legs() -> list[dict[str, str]]:
    """构建矩阵 `include:` 的每个 leg（逐条解析键值，注释行忽略）。"""
    after = _read(_WORKFLOW).split("\n        include:\n", 1)[1]
    kept: list[str] = []
    for line in after.splitlines():
        # `include:` 的条目缩进 10 空格；缩进回落到 matrix 同级即块结束。
        if line.strip() and not line.startswith(" " * 10):
            break
        if not line.strip().startswith("#"):
            kept.append(line)
    legs = [
        # YAML 里空值写作 `""`：去引号后再比对，否则 `rust_target` 会变成字面量 `""`。
        {k: v.strip().strip("\"'") for k, v in re.findall(r"(?m)^\s*([\w-]+):\s*(.*?)\s*$", chunk)}
        for chunk in re.split(r"(?m)^\s*- ", "\n".join(kept))[1:]
    ]
    assert legs, "未能从 build.yml 解析构建矩阵 leg"
    return legs


def uploaded_artifacts(block: str) -> set[str]:
    """job 正文里的工件名（保留 `${{ matrix.… }}` 模板）。

    排除步骤名（`- name:`）——只取 `upload-artifact` 的 `name:`。本仓工件名一律以
    `DSH Dock` 开头，故顺带把 job 级 `name: build (…)` 也排除在外。
    """
    return set(re.findall(r"(?m)^(?!\s*- )\s+name:\s*(DSH Dock.+?)\s*$", block))


def expand_artifact(name: str, leg: dict[str, str]) -> str:
    """把工件名里的 `${{ matrix.xxx }}` 按某个 leg 展开（未知键 → 空，同 YAML 隐式行为）。"""
    return re.sub(
        r"\$\{\{\s*matrix\.([\w-]+)\s*\}\}",
        lambda m: leg.get(m.group(1), ""),
        name,
    )


class PlatformWorld(NamedTuple):
    """一个必须被适配的「运行世界」及其两端工件名。"""

    label: str
    os_prefix: str
    rust_target: str
    installers: str
    updater: str


# 平台面契约（`AGENTS.md` §0 红线 3，2026-09-21 维护者裁定）。
# WSL2 是 **Windows leg 的运行时模式**（不单独占 leg，ADR-0016），故此处锁三平台四 leg。
PLATFORM_WORLDS = (
    PlatformWorld(
        "macOS arm64", "macos", "aarch64-apple-darwin",
        "DSH Dock-macOS", "DSH Dock-macOS-updater",
    ),
    PlatformWorld(
        "macOS x86_64", "macos", "x86_64-apple-darwin",
        "DSH Dock-macOS-x86_64", "DSH Dock-macOS-x86_64-updater",
    ),
    PlatformWorld(
        "Windows x64", "windows", "",
        "DSH Dock-Windows", "DSH Dock-Windows-updater",
    ),
    PlatformWorld(
        "Linux x64", "ubuntu", "",
        "DSH Dock-Linux", "DSH Dock-Linux-updater",
    ),
)


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


class PlatformCoverageTests(unittest.TestCase):
    """红线 3 的机器投影：矩阵 · 上传 · 下载三处齐备，缺一即红。

    红了的处理方式**不是**把本文件改绿：平台覆盖只增不减，收窄须维护者裁定 + ADR。
    """

    def _leg_for(self, world: PlatformWorld) -> dict[str, str] | None:
        for leg in matrix_legs():
            if (
                leg.get("os", "").startswith(world.os_prefix)
                and leg.get("rust_target", "") == world.rust_target
            ):
                return leg
        return None

    def test_every_platform_world_still_has_a_build_leg(self) -> None:
        """① 构建矩阵：三平台四 leg 一个都不许少。"""
        missing = [w.label for w in PLATFORM_WORLDS if self._leg_for(w) is None]
        self.assertFalse(
            missing,
            f"构建矩阵里已没有这些平台的 leg：{missing}。"
            f"平台覆盖不得擅自收窄（AGENTS.md §0 红线 3）；确需收窄 = 维护者裁定 + ADR。"
            f"现有 leg = {[leg.get('name') for leg in matrix_legs()]}",
        )

    def test_every_leg_uploads_installers_and_updater(self) -> None:
        """② 上传侧：每个 leg 都要产出安装器与 updater 工件（缺 = 该平台静默不出包）。"""
        templates = uploaded_artifacts(job_block("build"))
        for world in PLATFORM_WORLDS:
            leg = self._leg_for(world)
            if leg is None:
                continue  # 已由上一个用例精确报错
            produced = {expand_artifact(t, leg) for t in templates}
            self.assertTrue(
                {world.installers, world.updater} <= produced,
                f"{world.label} 的工件没有上传步骤：缺 "
                f"{sorted({world.installers, world.updater} - produced)}；"
                f"build job 现有上传工件 = {sorted(produced)}",
            )

    def test_release_job_publishes_every_world(self) -> None:
        """③ 发布侧：每个世界的安装器与 updater 都必须被下载进 Release。"""
        published = uploaded_artifacts(job_block("release"))
        for world in PLATFORM_WORLDS:
            self.assertTrue(
                {world.installers, world.updater} <= published,
                f"{world.label} 的产物没有被发布 job 下载：缺 "
                f"{sorted({world.installers, world.updater} - published)}；"
                f"release job 现有下载 = {sorted(published)}",
            )

    def test_release_publishes_exactly_what_the_matrix_builds(self) -> None:
        """**核心不变式**：构建出的工件集合 == 发布侧下载的工件集合。

        两个方向都要红：**漏发布**（矩阵建了、Release 不发 = 平台静默消失，用户看不见
        的降级）与**凭空下载**（Release 下发了没人建的产物 = 发布必然失败）。
        """
        built: set[str] = set()
        templates = uploaded_artifacts(job_block("build"))
        for leg in matrix_legs():
            built |= {expand_artifact(t, leg) for t in templates}
        published = uploaded_artifacts(job_block("release"))
        self.assertEqual(
            built,
            published,
            f"构建/发布工件集合不一致 —— 建了不发：{sorted(built - published)}；"
            f"发了没建：{sorted(published - built)}",
        )

    def test_no_build_leg_is_made_non_blocking(self) -> None:
        """构建 job 内不得有 `continue-on-error`：那等于留一个永远绿的红灯平台。"""
        values = re.findall(r"(?m)^\s*continue-on-error:\s*(.+?)\s*$", job_block("build"))
        offenders = [v for v in values if v.strip().strip("\"'").lower() != "false"]
        self.assertFalse(
            offenders,
            f"build job 出现 continue-on-error={offenders} —— 平台失败必须红，"
            f"不得非阻断（AGENTS.md §0 红线 3）",
        )


if __name__ == "__main__":
    unittest.main()
