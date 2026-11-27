# macOS 签名与公证（可选：Developer ID + Notarytool）

> **当前默认策略（2026-11-27 维护者裁定）**：CI 发布的 macOS 安装包默认采用 `ad-hoc` 签名
> （`src-tauri/tauri.conf.json` 的 `signingIdentity: "-"`），**不依赖 Apple Developer 账号与在线公证**，
> 避免开发者账号续费或 Apple 协议变更阻断发版流水线。应用内自动更新（`latest.json` + `.sig`）
> 使用 Tauri 自身的 `minisign` 密钥校验，不受影响；用户首次从浏览器打开 `.dmg` 需在
> 「系统设置 → 隐私与安全性」点击「仍要打开」。
>
> 如未来需要恢复正式签名与公证，在 GitHub 仓库 Settings → Secrets and variables → Actions →
> **Variables** 新增 `ENABLE_APPLE_SIGNING=true` 并确保下述 Secrets 完整即可。

以下文档说明如何配置可选的 `Developer ID Application 证书` 签名 + Apple 公证 + staple闭环：
未公证的 .app 在本机可跑，但**发给别人会被 Gatekeeper 首次拦截**（需手动在隐私与安全性中放行）。

本文告诉你**哪些必须你本人做一次**（涉及你的开发者账户凭据，无法自动化），
以及 CI 如何消费这些凭据。

---

## 你需要手动做的三件事（一次性）

### 1. 导出 Developer ID Application 证书（.p12）

1. 打开 **钥匙串访问**（Keychain Access）
2. 左侧选「我的证书」→ 找到 **Developer ID Application: <你的名字> (TEAMID)**
3. 右键 → 导出 → 存成 `.p12`（会要求设一个导出密码，自己记牢）
4. Base64 编码：

```bash
base64 -i YourCert.p12 | pbcopy   # 复制到剪贴板
```

> 没有这个证书？去 https://developer.apple.com/account/resources/certificates → 添加 →
> **Developer ID Application** → 按向导生成（需要你账户的证书签名请求，用钥匙串「证书助理
> → 从证书颁发机构请求证书」创建 CSR）。

### 2. 创建 App Store Connect API Key（公证用）

Apple 现在推荐用 **API Key** 而非账户密码公证：

1. 打开 https://appstoreconnect.apple.com → 用户与访问（Users and Access）→ 集成（Integrations）→ API Keys
2. 点 + 生成：角色选 **Developer**（要有 notarytool 权限），下载 `.p8` 文件（只下载这一次）
3. 记下三个值：
   - **Key ID**（页面上的 10 位字符）
   - **Issuer ID**（账号页顶部的 UUID）
   - `.p8` 文件内容

### 3. 把凭据存进 GitHub Actions Secrets

去 https://github.com/realguan/dsh-dock/settings/secrets/actions 添加：

| Secret | 值 |
| :--- | :--- |
| `APPLE_CERTIFICATE` | 第 1 步复制的 base64（**不含换行**） |
| `APPLE_CERTIFICATE_PASSWORD` | 第 1 步导出时设的密码 |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: 你的名字 (TEAMID)`（必填；与导入证书完全一致） |
| `APPLE_API_KEY` | 第 2 步 `.p8` 文件内容（workflow 仅在 tag 的 macOS 步骤写入临时文件） |
| `APPLE_API_ISSUER` | 第 2 步 Issuer ID |
| `APPLE_API_KEY_ID` | 第 2 步 Key ID |

> 当前 workflow 只接入 API Key 路径；推荐保留这一组，避免把 Apple 账号密码放进 CI。

---

## CI 怎么用它们

`.github/workflows/build.yml`：

1. **仅 tag 的 macOS 构建步骤**接收 Apple secrets，且**仅当仓库 Variables 中显式设置 `ENABLE_APPLE_SIGNING=true` 时才启用**；PR、分支构建、Windows/Linux 步骤，以及未开启该变量时的 tag 构建均会清空 `APPLE_*` 环境变量。开启后检测到任一 Apple secret，会要求证书、identity 和 API Key 整组完整，避免静默退化为 ad-hoc。
2. Tauri 读取 `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` 后自动导入临时 keychain；workflow 将 secret `APPLE_API_KEY` 的 `.p8` 内容短暂写入 runner 临时目录，并把 Tauri 所需的 `APPLE_API_KEY` 设为 `APPLE_API_KEY_ID`、`APPLE_API_KEY_PATH` 设为该临时路径。
3. `cargo tauri build`：
   - ta​​uri 用 `APPLE_SIGNING_IDENTITY` 给 `.app` 签名（Hardened Runtime + `entitlements.plist`）
   - 检测到 `APPLE_API_KEY` 组合 → 自动 **notarytool 公证 + staple**
4. 凭据完整且启用 `ENABLE_APPLE_SIGNING=true` 时，workflow 以 `codesign --verify --deep --strict` 和 `spctl --assess` 作为签名/公证闸门。
5. tag `v*` → release job 把 `.dmg` / updater 资产发到 GitHub Release。

**未启用 `ENABLE_APPLE_SIGNING=true`（当前默认）或未配 Apple secrets 时**：tag 构建会清掉
`APPLE_*` 变量，以 `tauri.conf.json` 的 `signingIdentity: "-"` 生成 ad-hoc 产物，并使用
`TAURI_SIGNING_PRIVATE_KEY` 签名自动更新包；用户首次从浏览器下载安装会被 Gatekeeper 提示，
在「系统设置 → 隐私与安全性」点击「仍要打开」即可。配置 `ENABLE_APPLE_SIGNING=true` 与完整证书/API Key
组合后才会升格为 Developer ID 签名并公证。

---

## 本地直接签名/公证（不想走 CI）

```bash
cd src-tauri
export APPLE_SIGNING_IDENTITY='Developer ID Application: 你的名字 (TEAMID)'
cargo tauri build
```

公证（本地需要 API key 文件）：

```bash
# Tauri 读这些 env 自动公证 + staple；APPLE_API_KEY 是 Key ID，不是 .p8 内容。
export APPLE_API_KEY=XXXXYYYYZZ
export APPLE_API_KEY_PATH=~/Downloads/AuthKey_XXXX.p8
export APPLE_API_ISSUER=... # issuer UUID
cargo tauri build
```

---

## 常见问题

- **`codesign` 报 "no identity found"**：证书没导出成功 / keychain 里无 Developer ID Application。用 `security find-identity -v -p codesigning` 自查。
- **公证失败 "invalid token"**：API Key 权限不足（要 Developer 角色）、Key ID / Issuer ID 不匹配，或 `APPLE_API_KEY_PATH` 未指向正确的 `.p8` 文件。
- **spctl --assess 不通过**：公证票没 staple 或验的是未公证的 intermediate build；CI 会将其视为 tag 发布失败。
- **首次公证可能要几十分钟**：Apple 服务器排队；`--skip-stapling` 可跳过等待（CI 已用默认等待，稳定出票）。
