// 最小 MCP stdio 服务器（仅供本机闸门用，零网络）：应答 initialize 与 tools/list。
import { createInterface } from "node:readline"
const rl = createInterface({ input: process.stdin })
rl.on("line", (line) => {
  let msg
  try {
    msg = JSON.parse(line)
  } catch {
    return
  }
  if (msg.method === "initialize") {
    process.stdout.write(
      JSON.stringify({
        jsonrpc: "2.0",
        id: msg.id,
        result: { protocolVersion: "2025-11-25", serverInfo: { name: "fake", version: "0" } },
      }) + "\n",
    )
  } else if (msg.method === "tools/list") {
    process.stdout.write(
      JSON.stringify({
        jsonrpc: "2.0",
        id: msg.id,
        result: { tools: [{ name: "alpha" }, { name: "beta" }] },
      }) + "\n",
    )
  } else if (msg.method === "resources/list") {
    process.stdout.write(
      JSON.stringify({ jsonrpc: "2.0", id: msg.id, result: { resources: [{ uri: "file:///x" }] } }) + "\n",
    )
  }
})
