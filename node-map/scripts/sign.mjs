#!/usr/bin/env node
// 对 map.json 做 ed25519 detached 签名（对文件原始字节签名，不做任何规范化）。
// 用法：node scripts/sign.mjs
// 私钥来源（二选一）：环境变量 NODE_MAP_SIGNING_KEY（CI）或 ./node-map-private.key（本地）。
import { createPrivateKey, sign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

const dir = new URL("..", import.meta.url);
const mapPath = new URL("map.json", dir);
const sigPath = new URL("map.json.sig", dir);

const keyHex = process.env.NODE_MAP_SIGNING_KEY
  ?? readFileSync(new URL("node-map-private.key", dir), "utf8").trim();
if (!/^[0-9a-f]{64}$/i.test(keyHex)) {
  console.error("私钥格式不对（需要 64 位 hex 的 ed25519 裸私钥）。");
  process.exit(1);
}

// pkcs8 DER 头（16 字节）+ 32 字节裸私钥 → Node 可识别的 KeyObject。
const pkcs8 = Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), Buffer.from(keyHex, "hex")]);
const keyObject = createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });

const mapBytes = readFileSync(mapPath);
const sig = sign(null, mapBytes, keyObject);
writeFileSync(sigPath, sig.toString("hex") + "\n");
console.log(`已签名 map.json（${mapBytes.length} 字节）→ map.json.sig`);
