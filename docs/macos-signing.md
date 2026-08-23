# macOS 签名与公证（Developer ID + Notarytool）

DSH Dock 的 macOS 分发闭环：`Developer ID Application 证书` 签名 + Apple 公证 + staple。
未签名/未公证的 .app 在本机可跑，但**发给别人会被 Gatekeeper 拦截**（"无法打开，因为无法验证开发者"）。

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
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: 你的名字 (TEAMID)`（可选，不设则自动取 keychain 中第一个 Developer ID） |
| `APPLE_API_KEY` | 第 2 步 .p8 文件内容（workflow 会自动写入临时文件并设 `APPLE_API_KEY_PATH`） |
| `APPLE_API_ISSUER` | 第 2 步 Issuer ID |
| `APPLE_API_KEY_ID` | 第 2 步 Key ID |
| `APPLE_TEAM_ID` | 你的 Team ID（登录 developer.apple.com → Membership 详情页可见） |

> 不想用 API Key？可改用应用专用密码两件套：
> `APPLE_ID`（Apple 账号邮箱）+ `APPLE_PASSWORD`（应用专用密码）+ `APPLE_TEAM_ID`。
> 推荐 API Key：更安全、独立于账号密码。

---

## CI 怎么用它们（已配好，无需你改）

`.github/workflows/build.yml`：

1. **macOS job** 检测到 `APPLE_CERTIFICATE` → 导入 .p12 到临时 keychain（`security import`，只给 codesign/notarytool 权限）
2. `cargo tauri build`：
   - ta​​uri 用 `APPLE_SIGNING_IDENTITY`（或自动取）给 `.app` 签名（Hardened Runtime + `entitlements.plist`）
   - 检测到 `APPLE_API_KEY` 组合 → 自动 **notarytool 公证 + staple**
3. 验证步骤：`codesign --verify --deep --strict` 确认签名有效
4. tag `v*` → release job 把带公证票的 .dmg / .zip 发到 GitHub Release

**没配 secrets 时**：签名步骤被 `if: env.APPLE_CERTIFICATE != ''` 跳过 → ad-hoc 签名，
构建照常出包（CI 可用、本机可跑），只是分发会被 Gatekeeper 拦。

---

## 本地直接签名/公证（不想走 CI）

```bash
cd src-tauri
cargo tauri build                     # signingIdentity 取本地 keychain 的 Developer ID
```

公证（本地需要 API key 文件）：

```bash
# tauri 读这些 env 自动公证 + staple
export APPLE_API_KEY="$(cat ~/Downloads/AuthKey_XXXX.p8)"
export APPLE_API_KEY_PATH=~/Downloads/AuthKey_XXXX.p8
export APPLE_API_ISSUER=... # issuer UUID
export APPLE_API_KEY_ID=... # 10 位 key id
cargo tauri build
```

---

## 常见问题

- **`codesign` 报 "no identity found"**：证书没导出成功 / keychain 里无 Developer ID Application。用 `security find-identity -v -p codesigning` 自查。
- **公证失败 "invalid token"**：API Key 权限不足（要 Developer 角色）或 `.p8` 内容有换行（`APPLE_API_KEY` 需单行）。
- **spctl --assess 不通过**：公证票没 staple 或验的是未公证的 intermediate build。正式发布以 tauri 自动 staple 为准，CI 里该步只记录不 gate。
- **首次公证可能要几十分钟**：Apple 服务器排队；`--skip-stapling` 可跳过等待（CI 已用默认等待，稳定出票）。