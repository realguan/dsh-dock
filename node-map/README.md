# @dsh-dock/node-map —— 签名的 Node 版本映射包

DSH Dock 的 Node 运行时版本映射，发布为 scoped npm 包（需要先创建 `dsh-dock` 这个 npm org，或在 `package.json` 里改用你自己的 scope 并同步 `src-tauri/src/updates.rs` 的 `NODE_MAP_PACKAGE`）。壳启动时从 registry 镜像链
（npmmirror → npmjs）拉取 `latest` 的 tarball，取出 `package/map.json` 与
`package/map.json.sig`，用**编译进壳的 ed25519 公钥**验签后采纳；任何失败
（拉不到 / 验签不过 / 内容不合法）都回退壳内置基线——**fail-closed**。

目的：升级 Node 版本不需要重新发壳。改 `map.json` → 签名 → `npm publish` 即可。

## 信任模型

签名不证明 Node 工件本身的正确性（SHA-256 值取自 nodejs.org 官方
`SHASUMS256.txt`），签名证明的是 **「DSH Dock 维护方背书这份映射」**——
防的是映射文件在分发链路上被篡改（指向恶意下载、伪哈希）。

## 更新流程

```bash
cd node-map
# 0. 首次：npm login + 创建 npm org `dsh-dock`（或把包名换成自己的 scope）
# 1. 改 map.json：nodeVersion + 六平台 sha256（抄 nodejs.org/dist/<v>/SHASUMS256.txt）
#    同时升 package.json 的 version，并确认 minShellVersion 覆盖存量壳
# 2. 签名（本地私钥或 CI 的 NODE_MAP_SIGNING_KEY）
node scripts/sign.mjs
# 3. 发布
npm publish --access public
```

## 密钥管理

- 首次：`node scripts/gen-key.mjs` → 私钥落 `node-map-private.key`（gitignore），
  公钥 hex 粘贴到 `src-tauri/src/updates.rs` 的 `NODE_MAP_PUBKEY_HEX`。
- 私钥只应存在于：本地密钥文件（不提交）与 GitHub Secret
  `NODE_MAP_SIGNING_KEY`（CI 发布用）。任何机器不得长期持有。
- 轮换：生成新对 → 壳内更新公钥并发版 → 旧密钥销毁。轮换窗口内新旧映射
  同时有效（旧壳认旧公钥，直到它们升级）。
