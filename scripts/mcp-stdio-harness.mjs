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
// 请求：{ command, args[], cwd?, envPath?, stdin?, steps?, timeoutMs }
//
// **两种模式**：
//   ① `stdin`：一次性把整段文本喂进去（适合无握手的普通进程）。
//   ② `steps`：**脚本化对话** —— 由宿主给出步骤序列，脚本逐步执行：
//        [{ write: "<一行>" }, { awaitId: 1 }, { write: "..." }, { awaitId: 2 }, ...]
//      为什么必须有它：MCP 握手**有序**（initialize ⇒ 等响应 ⇒ initialized ⇒ 才能 list）。
//      一次性喂完等于并发发请求，违反协议顺序，部分服务器会直接忽略后续请求。
//      但"等待哪个 id"这类知识仍属**宿主**（步骤由宿主生成）—— 脚本只认 write / awaitId，
//      零 MCP 知识，故不存在第二份协议实现。
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
    if (!line.trim()) continue
    stdout.push(Buffer.from(line, "utf8").toString("base64"))
    try {
      const msg = JSON.parse(line)
      if (typeof msg.id === "number") {
        seenIds.add(msg.id)
        const resolve = pending.get(msg.id)
        if (resolve) {
          pending.delete(msg.id)
          resolve()
        }
      }
    } catch {
      // 非 JSON 行（服务器的日志）：照样收进 stdout，交给宿主判断
    }
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

// ---- 投递：脚本化对话（steps）或一次性喂入（stdin） ----
const pending = new Map() // id -> resolve，供 awaitId 等待
const seenIds = new Set()

function awaitId(id, deadline) {
  if (seenIds.has(id)) return Promise.resolve()
  return new Promise((resolve, reject) => {
    pending.set(id, resolve)
    const left = deadline - Date.now()
    if (left <= 0) return reject(new Error(`等待 id=${id} 超时`))
    setTimeout(() => {
      if (pending.delete(id)) reject(new Error(`等待 id=${id} 超时`))
    }, left)
  })
}

async function runSteps(steps, deadline) {
  for (const step of steps) {
    if (Date.now() > deadline) throw new Error("步骤执行超时")
    if (typeof step.write === "string") {
      child.stdin.write(step.write.endsWith("\n") ? step.write : step.write + "\n")
    } else if (typeof step.awaitId === "number") {
      await awaitId(step.awaitId, deadline)
    } else {
      throw new Error(`未知步骤：${JSON.stringify(step)}`)
    }
  }
  child.stdin.end()
}

if (Array.isArray(req.steps)) {
  runSteps(req.steps, started + timeoutMs).catch((e) =>
    fail(`对话失败：${e.message}`, { elapsedMs: Date.now() - started, stderrTail }),
  )
} else {
  try {
    child.stdin.write(req.stdin ?? "")
    child.stdin.end()
  } catch (e) {
    fail(`写入 stdin 失败：${e.message}`, { elapsedMs: Date.now() - started })
  }
}
