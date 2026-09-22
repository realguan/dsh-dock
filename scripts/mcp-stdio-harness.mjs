// mcp-stdio-harness.mjs —— **通用** stdio 子进程搬运器（2026-09-21，WSL 客体档 MCP 探测用）。
//
// 为什么是"通用搬运"而不是"客体版探测"：MCP 协议件（initialize / initialized / list 请求与
// 响应解析）在本仓**已经是纯函数**（`src-tauri/src/mcp_probe.rs`：`initialize_request` /
// `initialized_notification` / `list_request` / `parse_rpc_envelope` / `parse_named_list`）。
// 若在客体侧再写一份协议实现，就会造出**第二个内核**（违 AGENTS §6「宿主/客体同一内核」，
// 且两侧必然漂移）。故本脚本**零 MCP 知识**：只负责"起进程、喂 stdin、收 stdout"，
// 协议请求由宿主生成、响应由宿主解析。
//
// 用法：node mcp-stdio-harness.mjs <base64(JSON 请求)>
// 请求：{ command, args[], cwd?, envPath?, stdin, timeoutMs }
// 输出：**单行 JSON**（下游按行解析，与 guest 其它脚本同口径）：
//   { ok: true, code, elapsedMs, stdoutB64: [...], stderrTail }
//   { ok: false, error, code?, elapsedMs, stderrTail }
import { spawn } from "node:child_process"

function decode(b64) {
  return Buffer.from(b64, "base64").toString("utf8")
}

function fail(error, extra = {}) {
  process.stdout.write(JSON.stringify({ ok: false, error, ...extra }) + "\n")
  process.exit(0) // 恒 0 退出：成败看结果行（非零退出会被上层折叠成"无输出"，丢诊断）
}

const raw = process.argv[2]
if (!raw) fail("缺少请求参数（base64 JSON）")

let req
try {
  req = JSON.parse(decode(raw))
} catch (e) {
  fail(`请求解析失败：${e.message}`)
}

const started = Date.now()
let child
try {
  const env = { ...process.env }
  if (req.envPath) env.PATH = req.envPath // 复现 dsh 子进程的解析条件（宿主传入）
  child = spawn(req.command, req.args ?? [], {
    cwd: req.cwd || undefined,
    env,
    stdio: ["pipe", "pipe", "pipe"],
  })
} catch (e) {
  fail(`启动失败：${e.message}`, { elapsedMs: Date.now() - started })
}

const stdout = []
let stderrTail = ""
child.stdout.on("data", (d) => {
  for (const line of d.toString("utf8").split("\n")) {
    if (line.trim()) stdout.push(Buffer.from(line, "utf8").toString("base64"))
  }
})
child.stderr.on("data", (d) => {
  stderrTail = (stderrTail + d.toString("utf8")).slice(-2000)
})

const timeoutMs = req.timeoutMs ?? 15000
const timer = setTimeout(() => {
  try {
    child.kill("SIGKILL")
  } catch {}
  fail("超时：服务器未在时限内完成响应", {
    elapsedMs: Date.now() - started,
    stderrTail,
  })
}, timeoutMs)

child.on("error", (e) => {
  clearTimeout(timer)
  fail(`进程错误：${e.message}`, { elapsedMs: Date.now() - started, stderrTail })
})
child.on("close", (code) => {
  clearTimeout(timer)
  process.stdout.write(
    JSON.stringify({
      ok: true,
      code,
      elapsedMs: Date.now() - started,
      stdoutB64: stdout,
      stderrTail,
    }) + "\n",
  )
  process.exit(0)
})

// 一次性喂完（JSON-RPC 在 stdio 上按序处理）：宿主已把 initialize / initialized /
// list 请求按序拼好，这里只负责写进去并关掉 stdin（多数服务器据此判定"客户端说完了"）。
try {
  child.stdin.write(req.stdin ?? "")
  child.stdin.end()
} catch (e) {
  fail(`写入 stdin 失败：${e.message}`, { elapsedMs: Date.now() - started })
}
