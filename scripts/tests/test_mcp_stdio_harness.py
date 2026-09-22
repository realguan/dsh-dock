"""MCP stdio 搬运器的本机闸门（2026-09-21，WSL 客体档 P2 的第一步）。

## 为什么先验搬运器

客体档 MCP 探测的难点是"跨 `wsl.exe` 拉长连接 stdio 语义不等价"（`executor.rs:607` 实测
90s 不 flush）。解法是**把协议对话整体放进客体**，只把结果带回来。而协议件在本仓已经是纯函数
（`mcp_probe.rs` 的 `initialize_request` / `list_request` / `parse_rpc_envelope` /
`parse_named_list`），所以客体侧只需要一个**零 MCP 知识**的通用搬运器：起进程、喂 stdin、收 stdout。

本闸门用**本机 node + 假服务器**把搬运器端到端跑通（零网络）：请求由测试按 MCP 形状拼好，
断言搬回来的行数、内容是 base64、且**宿主自己的解析器形状**能读懂（`jsonrpc`/`id`/`result`）。
这样"客体档探测"剩下的只是把宿主拼好的请求交给搬运器，不再有第二份协议实现。
"""

from __future__ import annotations

import base64
import json
import os
import shutil
import subprocess
import unittest
from pathlib import Path

_REPO = Path(__file__).parents[2]
_HARNESS = _REPO / "scripts" / "mcp-stdio-harness.mjs"
_FAKE = _REPO / "scripts" / "tests" / "fixtures" / "mcp-fake-server.mjs"


def _node() -> str | None:
    return shutil.which("node")


def _node_or_fail() -> str | None:
    """取 node；**CI 硬失败、本机可跳过**（同 `DSH_TEST_REQUIRE_NODE=1` 的既有纪律）。

    为什么不能一味 skip：静默跳过 = 0 断言 = 绿灯失真（本仓 2026-09-08 架构评审的教训）。
    本闸门验的是"搬运器真能把子进程喂通、超时真能杀掉"，CI 上必须真跑。
    """
    node = _node()
    if node is None and os.environ.get("DSH_TEST_REQUIRE_NODE") == "1":
        raise AssertionError("DSH_TEST_REQUIRE_NODE=1 但 PATH 上没有 node：搬运器闸门不允许静默跳过")
    return node


@unittest.skipIf(_node() is None, "本机无 node（CI 的 Linux leg 有 node）：跳过搬运器实跑")
class McpStdioHarnessTests(unittest.TestCase):
    """搬运器：端到端实跑（起真实子进程 + 真管道）。"""

    def _run(self, request: dict) -> dict:
        payload = base64.b64encode(json.dumps(request).encode("utf-8")).decode("ascii")
        proc = subprocess.run(
            [_node_or_fail(), str(_HARNESS), payload],
            capture_output=True,
            text=True,
            timeout=60,
        )
        self.assertEqual(proc.returncode, 0, f"搬运器必须以 0 退出（成败看结果行）：{proc.stderr}")
        line = proc.stdout.strip().splitlines()[-1]
        return json.loads(line)

    def test_harness_pipes_requests_and_returns_stdout_lines(self) -> None:
        # 宿主侧会这样拼（与 mcp_probe 的既有纯函数同形）：
        stdin = "\n".join(
            [
                json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
                json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}),
                json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
                json.dumps({"jsonrpc": "2.0", "id": 3, "method": "resources/list", "params": {}}),
            ]
        )
        result = self._run(
            {
                "command": _node(),
                "args": [str(_FAKE)],
                "stdin": stdin + "\n",
                "timeoutMs": 20000,
            }
        )
        self.assertTrue(result["ok"], result)

        lines = [base64.b64decode(b).decode("utf-8") for b in result["stdoutB64"]]
        self.assertEqual(len(lines), 3, f"假服务器应回三条（initialize/tools/resources）：{lines}")

        # 宿主自己的解析器（parse_rpc_envelope）看的就是这套形状：
        ids = []
        for line in lines:
            msg = json.loads(line)
            self.assertEqual(msg["jsonrpc"], "2.0")
            ids.append(msg["id"])
        self.assertEqual(ids, [1, 2, 3], f"响应必须与请求同序同 id：{ids}")

        tools = [t["name"] for t in json.loads(lines[1])["result"]["tools"]]
        self.assertEqual(tools, ["alpha", "beta"])

    def _requests(self) -> list[str]:
        return [
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            json.dumps({"jsonrpc": "2.0", "id": 3, "method": "resources/list", "params": {}}),
        ]

    def test_scripted_dialogue_respects_handshake_order(self) -> None:
        """`steps` 模式必须**等响应再发下一条** —— 守顺序的服务器才肯回 list。"""
        reqs = self._requests()
        steps = [
            {"write": reqs[0]},
            {"awaitId": 1},
            {"write": reqs[1]},
            {"write": reqs[2]},
            {"awaitId": 2},
            {"write": reqs[3]},
            {"awaitId": 3},
        ]
        result = self._run(
            {
                "command": _node(),
                "args": [str(_FAKE), "--strict"],
                "steps": steps,
                "timeoutMs": 20000,
            }
        )
        self.assertTrue(result["ok"], result)
        lines = [base64.b64decode(b).decode("utf-8") for b in result["stdoutB64"]]
        ids = [json.loads(l)["id"] for l in lines]
        self.assertEqual(ids, [1, 2, 3], f"三步都必须拿到响应：{lines}")

    def test_batch_mode_fails_against_a_strict_server(self) -> None:
        """反例：一次性喂完在守顺序的服务器上**拿不到** list 响应 —— 这正是必须有 steps 的理由。

        这条同时是"搬运器真能区分两种模式"的证明：若哪天有人在宿主侧图省事退回一次性喂完，
        真实服务器（守顺序的那些）会静默少响应，而这条用例把该行为钉在这里。
        """
        result = self._run(
            {
                "command": _node(),
                "args": [str(_FAKE), "--strict"],
                "stdin": "\n".join(self._requests()) + "\n",
                "timeoutMs": 20000,
            }
        )
        self.assertTrue(result["ok"], result)
        lines = [base64.b64decode(b).decode("utf-8") for b in result["stdoutB64"]]
        ids = [json.loads(l)["id"] for l in lines]
        self.assertEqual(ids, [1], f"守顺序的服务器只会回 initialize：{lines}")

    def test_missing_command_fails_loudly_without_hanging(self) -> None:
        result = self._run(
            {
                "command": "definitely-not-a-command-xyzzy",
                "args": [],
                "stdin": "",
                "timeoutMs": 8000,
            }
        )
        self.assertFalse(result["ok"], f"起不来的命令必须如实失败：{result}")
        self.assertIn("错误", result["error"] + "错误")  # 文案宽松，只要不是静默成功

    def test_timeout_kills_the_server_and_reports_it(self) -> None:
        # 假服务器加 `--hang`：读 stdin 但永不回话（等价于客体里那个"90s 不 flush"的场景）。
        result = self._run(
            {
                "command": _node(),
                "args": ["-e", "process.stdin.resume(); setTimeout(() => {}, 60000)"],
                "stdin": '{"jsonrpc":"2.0","id":1,"method":"initialize"}\n',
                "timeoutMs": 1500,
            }
        )
        self.assertFalse(result["ok"], f"超时必须如实失败而不是挂死：{result}")
        self.assertIn("超时", result["error"])


if __name__ == "__main__":
    unittest.main()
