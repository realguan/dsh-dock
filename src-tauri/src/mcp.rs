//! mcp.rs —— Profile 的 MCP 服务器结构化管理（4.7）。
//!
//! 契约与防坑准则（roadmap §1 & 4.7 审核意见）：
//! 1. Cordis Patch 对 `config` 键是整体替换（Replace，无深合并）；
//! 2. 增删改单个 MCP Server 时，必须先在内存中构建包含全部 servers 的完整 `config.mcpServers` 对象，
//!    再整行更新写回 `cordis.patch.yml`；
//! 3. 保持原子写入与原有其它插件 patch 顺序不变。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const MCP_CLIENT_PKG: &str = "@deepseek-ai/dsh-mcp-client";
const PROFILE_PATCH_FILENAME: &str = "cordis.patch.yml";

/// MCP 服务器配置项（前后端交互契约）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
}

/// 读取指定 profile 的全部 MCP 服务器配置
pub fn list_mcp_servers(home: &Path, profile: &str) -> Result<Vec<McpServerConfig>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let patch_path = home
        .join("profiles")
        .join(profile)
        .join(PROFILE_PATCH_FILENAME);
    if !patch_path.is_file() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&patch_path)
        .map_err(|e| format!("读取 cordis.patch.yml 失败：{e}"))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let entries: Vec<serde_json::Value> = serde_yaml::from_str(&content)
        .map_err(|e| format!("解析 cordis.patch.yml YAML 失败：{e}"))?;

    let mut result = Vec::new();
    for entry in entries {
        if entry.get("package").and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG) {
            if let Some(config) = entry.get("config") {
                if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                    for (name, srv_val) in servers {
                        let cmd = srv_val
                            .get("command")
                            .and_then(|c| c.as_str())
                            .unwrap_or("npx")
                            .to_string();
                        let args = srv_val
                            .get("args")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let mut env_map = BTreeMap::new();
                        if let Some(env_obj) = srv_val.get("env").and_then(|e| e.as_object()) {
                            for (k, v) in env_obj {
                                if let Some(vs) = v.as_str() {
                                    env_map.insert(k.clone(), vs.to_string());
                                }
                            }
                        }
                        let disabled = srv_val
                            .get("disabled")
                            .and_then(|d| d.as_bool())
                            .unwrap_or(false);

                        result.push(McpServerConfig {
                            name: name.clone(),
                            command: cmd,
                            args,
                            env: env_map,
                            disabled,
                        });
                    }
                }
            }
        }
    }

    Ok(result)
}

/// 保存或更新单个 MCP 服务器配置（整体替换写回）
pub fn save_mcp_server(home: &Path, profile: &str, server: McpServerConfig) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    if server.name.trim().is_empty() {
        return Err("MCP 服务器名称不能为空".to_string());
    }
    if server.name.contains('/') || server.name.contains('\\') || server.name.contains(' ') {
        return Err(format!(
            "MCP 服务器名称「{}」包含非法字符（禁空格/斜杠）",
            server.name
        ));
    }

    let profile_dir = home.join("profiles").join(profile);
    if !profile_dir.is_dir() {
        return Err(format!("profile「{profile}」不存在或尚未物化"));
    }

    let patch_path = profile_dir.join(PROFILE_PATCH_FILENAME);
    // 写入统一走 `plugins::PatchFile`（2026-09-11，task-26）：**未改动的条目逐字节
    // 原样回填**（保住用户行间注释、键序、缩进），只有本函数改写的 MCP 条目重新
    // 序列化；写入前备份 + 原子替换。不得在此另写一份文件写入逻辑——两套写入器
    // 曾漂移成「一边丢注释、一边不备份」，注释丢失不可逆。
    let mut patch = if patch_path.is_file() {
        crate::plugins::PatchFile::read(&patch_path)?
    } else {
        crate::plugins::PatchFile::empty()
    };

    // 准备要插入/更新的 server YAML 映射（`serde_yaml::Mapping` 基于 IndexMap，
    // **插入序保真**——旧实现经 `serde_json::Value`（BTreeMap）中转，键序被字典序打乱）。
    let mut server_map = serde_yaml::Mapping::new();
    server_map.insert(
        serde_yaml::Value::String("command".into()),
        serde_yaml::Value::String(server.command),
    );
    server_map.insert(
        serde_yaml::Value::String("args".into()),
        serde_yaml::Value::Sequence(
            server
                .args
                .into_iter()
                .map(serde_yaml::Value::String)
                .collect(),
        ),
    );
    if !server.env.is_empty() {
        let mut env_map = serde_yaml::Mapping::new();
        for (k, v) in server.env {
            env_map.insert(serde_yaml::Value::String(k), serde_yaml::Value::String(v));
        }
        server_map.insert(
            serde_yaml::Value::String("env".into()),
            serde_yaml::Value::Mapping(env_map),
        );
    }
    if server.disabled {
        server_map.insert(
            serde_yaml::Value::String("disabled".into()),
            serde_yaml::Value::Bool(true),
        );
    }
    let server_value = serde_yaml::Value::Mapping(server_map);
    let server_name = server.name;

    // 语义不变（Cordis Patch 对 `config` 键是整体替换）：整段重建 `config.mcpServers`。
    let mut found = false;
    patch.for_each_entry_mut(|_, entry| {
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if m.get("package").and_then(|p| p.as_str()) != Some(MCP_CLIENT_PKG) {
            return false;
        }
        found = true;
        let cfg_key = serde_yaml::Value::String("config".into());
        if !m.contains_key(&cfg_key) {
            m.insert(
                cfg_key.clone(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            );
        }
        let Some(cfg) = m.get_mut(&cfg_key).and_then(|c| c.as_mapping_mut()) else {
            return false;
        };
        let servers_key = serde_yaml::Value::String("mcpServers".into());
        if !cfg.contains_key(&servers_key) {
            cfg.insert(
                servers_key.clone(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            );
        }
        let Some(servers) = cfg.get_mut(&servers_key).and_then(|s| s.as_mapping_mut()) else {
            return false;
        };
        servers.insert(
            serde_yaml::Value::String(server_name.clone()),
            server_value.clone(),
        );
        true
    });

    if !found {
        // 新增一个 MCP 插件 entry
        let mut servers_map = serde_yaml::Mapping::new();
        servers_map.insert(serde_yaml::Value::String(server_name), server_value);
        let mut cfg = serde_yaml::Mapping::new();
        cfg.insert(
            serde_yaml::Value::String("mcpServers".into()),
            serde_yaml::Value::Mapping(servers_map),
        );
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(
            serde_yaml::Value::String("id".into()),
            serde_yaml::Value::String("mcp".into()),
        );
        entry.insert(
            serde_yaml::Value::String("package".into()),
            serde_yaml::Value::String(MCP_CLIENT_PKG.into()),
        );
        entry.insert(
            serde_yaml::Value::String("config".into()),
            serde_yaml::Value::Mapping(cfg),
        );
        patch.push(serde_yaml::Value::Mapping(entry));
    }

    patch.write(&patch_path)?;

    Ok(())
}

/// 删除指定 MCP 服务器配置
pub fn delete_mcp_server(home: &Path, profile: &str, server_name: &str) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let profile_dir = home.join("profiles").join(profile);
    let patch_path = profile_dir.join(PROFILE_PATCH_FILENAME);
    if !patch_path.is_file() {
        return Ok(());
    }

    let mut patch = crate::plugins::PatchFile::read(&patch_path)?;

    // 与 save 同源写入器；只有真正删掉键才回写（无改动不产生备份 / mtime 抖动）。
    let servers_key = serde_yaml::Value::String("mcpServers".into());
    let cfg_key = serde_yaml::Value::String("config".into());
    let name_key = serde_yaml::Value::String(server_name.to_string());
    let mut removed = false;
    patch.for_each_entry_mut(|_, entry| {
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if m.get("package").and_then(|p| p.as_str()) != Some(MCP_CLIENT_PKG) {
            return false;
        }
        let Some(servers) = m
            .get_mut(&cfg_key)
            .and_then(|c| c.as_mapping_mut())
            .and_then(|c| c.get_mut(&servers_key))
            .and_then(|s| s.as_mapping_mut())
        else {
            return false;
        };
        if servers.remove(&name_key).is_some() {
            removed = true;
            return true;
        }
        false
    });

    if !removed {
        return Ok(());
    }
    patch.write(&patch_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_crud_flow_on_patch_yaml() {
        let tmp = std::env::temp_dir().join(format!("dsh-mcp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();

        // 初始为空
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert!(list.is_empty());

        // 保存一个 GitHub MCP 服务
        let mut env = BTreeMap::new();
        env.insert("TOKEN".to_string(), "abc".to_string());
        let srv = McpServerConfig {
            name: "github".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
            env,
            disabled: false,
        };
        save_mcp_server(&tmp, "testprof", srv.clone()).unwrap();

        // 列表应包含 1 个
        let list2 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].name, "github");
        assert_eq!(list2[0].command, "npx");
        assert_eq!(list2[0].env.get("TOKEN").map(String::as_str), Some("abc"));

        // 再添加一个 Postgres 服务
        let srv2 = McpServerConfig {
            name: "postgres".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-postgres".to_string(),
            ],
            env: BTreeMap::new(),
            disabled: true,
        };
        save_mcp_server(&tmp, "testprof", srv2).unwrap();

        let list3 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list3.len(), 2);

        // 删除 github 服务
        delete_mcp_server(&tmp, "testprof", "github").unwrap();
        let list4 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list4.len(), 1);
        assert_eq!(list4[0].name, "postgres");
        assert!(list4[0].disabled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mcp_preserves_existing_non_mcp_patch_entries() {
        let tmp = std::env::temp_dir().join(format!("dsh-mcp-preserve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();

        // 预设一个包含其它插件的 cordis.patch.yml
        let initial_patch = r#"
- id: custom-plugin
  package: "@custom/plugin-demo"
  config:
    apiKey: "secret-123"
"#;
        std::fs::write(prof_dir.join("cordis.patch.yml"), initial_patch).unwrap();

        // 添加一个 MCP 服务
        let srv = McpServerConfig {
            name: "fs".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "server-fs".to_string()],
            env: BTreeMap::new(),
            disabled: false,
        };
        save_mcp_server(&tmp, "testprof", srv).unwrap();

        // 读取 patch 内容，确保 custom-plugin 仍然存在
        let patch_content = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(patch_content.contains("@custom/plugin-demo"));
        assert!(patch_content.contains("secret-123"));
        assert!(patch_content.contains(MCP_CLIENT_PKG));
        assert!(patch_content.contains("server-fs"));

        // 再更新这个 MCP 服务
        let srv_updated = McpServerConfig {
            name: "fs".to_string(),
            command: "uvx".to_string(),
            args: vec!["mcp-fs".to_string()],
            env: BTreeMap::new(),
            disabled: true,
        };
        save_mcp_server(&tmp, "testprof", srv_updated).unwrap();

        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].command, "uvx");
        assert!(list[0].disabled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 带注释的 patch fixture：头部注释块 + **段间注释** + 一个非 MCP 条目 +
    /// 一个 MCP 条目。注释是用户人工资产（YAML 注释无程序语义，丢了不可逆）。
    const PATCH_WITH_COMMENTS: &str = "\
# ======== 用户手写说明（不可丢）========
# 第二行头部注释
- id: custom-plugin
  package: \"@custom/plugin-demo\"
  config:
    apiKey: \"secret-123\"
# 段间注释：MCP 段从此开始（行间，非头部）
- id: mcp
  package: \"@deepseek-ai/dsh-mcp-client\"
  config:
    mcpServers:
      fs:
        command: npx
        args:
          - -y
          - server-fs
";

    fn mcp_fixture(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();
        std::fs::write(prof_dir.join("cordis.patch.yml"), PATCH_WITH_COMMENTS).unwrap();
        (tmp, prof_dir)
    }

    fn server(name: &str, command: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            command: command.to_string(),
            args: vec!["-y".to_string(), format!("server-{name}")],
            env: BTreeMap::new(),
            disabled: false,
        }
    }

    /// **复现（2026-09-11，task-26）**：MCP 写入曾对 `cordis.patch.yml` 做整数组
    /// 重序列化 ⇒ **注释全丢**（头部与段间都丢，YAML 注释是用户人工资产，不可逆）；
    /// 且**无备份**。本用例坐实「注释必须保住」。
    #[test]
    fn mcp_write_preserves_user_comments() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-comments");

        save_mcp_server(&tmp, "testprof", server("github", "npx")).unwrap();

        let after = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(
            after.contains("# ======== 用户手写说明（不可丢）========"),
            "头部注释块必须保住，实测内容：\n{after}"
        );
        assert!(
            after.contains("# 第二行头部注释"),
            "头部注释第二行必须保住，实测内容：\n{after}"
        );
        assert!(
            after.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "段间（行间）注释必须保住——只保头部不够，实测内容：\n{after}"
        );
        // 功能不得回归：MCP 增删改照旧生效
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 2, "fs + github 都应在：{list:?}");
        assert!(list.iter().any(|s| s.name == "fs"));
        assert!(list.iter().any(|s| s.name == "github"));
        // 非 MCP 条目原样保留
        assert!(after.contains("@custom/plugin-demo"));
        assert!(after.contains("secret-123"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 删除路径同源：`delete_mcp_server` 同样不得丢注释（同一修复必须覆盖两条路径）。
    #[test]
    fn mcp_delete_preserves_user_comments() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-comments-del");

        delete_mcp_server(&tmp, "testprof", "fs").unwrap();

        let after = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(
            after.contains("# ======== 用户手写说明（不可丢）========"),
            "删除路径同样必须保住头部注释：\n{after}"
        );
        assert!(
            after.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "删除路径同样必须保住段间注释：\n{after}"
        );
        assert!(!list_mcp_servers(&tmp, "testprof")
            .unwrap()
            .iter()
            .any(|s| s.name == "fs"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// **无备份**：覆写用户数据前必须留一份 `.bak-<unix秒>`（复用
    /// `fs_backup::backup_before_overwrite`，AGENTS §6 已登记），且备份内容 =
    /// 覆写前原文。
    #[test]
    fn mcp_write_backs_up_before_overwrite() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-backup");

        save_mcp_server(&tmp, "testprof", server("github", "npx")).unwrap();

        let backups: Vec<std::path::PathBuf> = std::fs::read_dir(&prof_dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("cordis.patch.yml.bak-"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(backups.len(), 1, "覆写前应留一份备份：{backups:?}");
        assert_eq!(
            std::fs::read_to_string(&backups[0]).unwrap(),
            PATCH_WITH_COMMENTS,
            "备份内容必须是覆写前原文（含注释）"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
