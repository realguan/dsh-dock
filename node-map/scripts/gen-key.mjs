#!/usr/bin/env node
// 生成 node-map 签名密钥对（仅首次需要；公钥钉进壳，私钥进 CI secret）。
// 用法：node scripts/gen-key.mjs
// 输出：私钥写入 ./node-map-private.key（已 gitignore），公钥 hex 打印到终端——
//       粘贴到 src-tauri/src/updates.rs 的 NODE_MAP_PUBKEY_HEX。
import { generateKeyPairSync } from "node:crypto";
import { writeFileSync } from "node:fs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubRaw = publicKey.export({ type: "spki", format: "der" }).subarray(12);
const privRaw = privateKey.export({ type: "pkcs8", format: "der" }).subarray(16);

writeFileSync(new URL("../node-map-private.key", import.meta.url), privRaw.toString("hex") + "\n");

console.log("私钥已写入 node-map/node-map-private.key（gitignore 内，切勿提交）。");
console.log("请把它配置为 GitHub Secret NODE_MAP_SIGNING_KEY，并在轮换密钥时同步更新壳内公钥。");
console.log("\nPUBKEY_HEX=" + pubRaw.toString("hex"));
