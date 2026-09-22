// 最小 MCP stdio 服务器（仅供本机闸门用，零网络）：应答 initialize 与 tools/list。
import { createInterface } from "node:readline"
const rl = createInterface({ input: process.stdin })
// `--strict`：模拟**守协议顺序**的真实服务器 —— 未完成 initialize 握手前到达的 list 请求
// 一律忽略（这正是"一次性喂完"会踩的坑）。
const strict = process.argv.includes("--strict")
// `--ignore-templates`：模拟"合法地什么都不回"（可选项不受支持时的真实行为之一）。
const ignoreTemplates = process.argv.includes("--ignore-templates")
let handshaken = false        // 收到过 initialize
let respondedInit = false     // **已把 initialize 的响应写出去**（真实服务器的判据是这个）
rl.on("line", (line) => {
  let msg
  try {
    msg = JSON.parse(line)
  } catch {
    return
  }
  if (msg.method === "initialize") {
    handshaken = true
    if (strict) {
      // 真实服务器：握手响应**不是立刻**回的（要起进程/初始化）。期间到达的 list 请求
      // 一律忽略 —— 这正是"一次性喂完"会踩的坑。
      setTimeout(() => {
        respondedInit = true
        process.stdout.write(
          JSON.stringify({
            jsonrpc: "2.0",
            id: msg.id,
            result: { protocolVersion: "2025-11-25", serverInfo: { name: "fake", version: "0" } },
          }) + "\n",
        )
      }, 300)
      return
    }
    process.stdout.write(
      JSON.stringify({
        jsonrpc: "2.0",
        id: msg.id,
        result: { protocolVersion: "2025-11-25", serverInfo: { name: "fake", version: "0" } },
      }) + "\n",
    )
  } else if (msg.method === "tools/list") {
    if (strict && !respondedInit) return // 握手响应未回 ⇒ 忽略（不回任何东西）
    process.stdout.write(
      JSON.stringify({
        jsonrpc: "2.0",
        id: msg.id,
        result: { tools: [{ name: "alpha" }, { name: "beta" }] },
      }) + "\n",
    )
  } else if (msg.method === "resources/templates/list") {
    if (ignoreTemplates) return
  } else if (msg.method === "resources/list") {
    if (strict && !respondedInit) return
    process.stdout.write(
      JSON.stringify({ jsonrpc: "2.0", id: msg.id, result: { resources: [{ uri: "file:///x" }] } }) + "\n",
    )
  }
})
