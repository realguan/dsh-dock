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
    let patch_path = home.join("profiles").join(profile).join(PROFILE_PATCH_FILENAME);
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
pub fn save_mcp_server(
    home: &Path,
    profile: &str,
    server: McpServerConfig,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    if server.name.trim().is_empty() {
        return Err("MCP 服务器名称不能为空".to_string());
    }
    if server.name.contains('/') || server.name.contains('\\') || server.name.contains(' ') {
        return Err(format!("MCP 服务器名称「{}」包含非法字符（禁空格/斜杠）", server.name));
    }

    let profile_dir = home.join("profiles").join(profile);
    if !profile_dir.is_dir() {
        return Err(format!("profile「{profile}」不存在或尚未物化"));
    }

    let patch_path = profile_dir.join(PROFILE_PATCH_FILENAME);
    let content = if patch_path.is_file() {
        std::fs::read_to_string(&patch_path)
            .map_err(|e| format!("读取 cordis.patch.yml 失败：{e}"))?
    } else {
        String::new()
    };

    let mut entries: Vec<serde_json::Value> = if content.trim().is_empty() {
        Vec::new()
    } else {
        serde_yaml::from_str(&content)
            .map_err(|e| format!("解析 cordis.patch.yml 失败：{e}"))?
    };

    // 查找已有的 MCP client entry
    let mut found_index = None;
    for (i, entry) in entries.iter().enumerate() {
        if entry.get("package").and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG) {
            found_index = Some(i);
            break;
        }
    }

    // 准备要插入/更新的 server JSON 对象
    let mut server_obj = serde_json::Map::new();
    server_obj.insert(
        "command".to_string(),
        serde_json::Value::String(server.command),
    );
    server_obj.insert(
        "args".to_string(),
        serde_json::Value::Array(
            server
                .args
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    if !server.env.is_empty() {
        let mut env_obj = serde_json::Map::new();
        for (k, v) in server.env {
            env_obj.insert(k, serde_json::Value::String(v));
        }
        server_obj.insert("env".to_string(), serde_json::Value::Object(env_obj));
    }
    if server.disabled {
        server_obj.insert("disabled".to_string(), serde_json::Value::Bool(true));
    }

    if let Some(idx) = found_index {
        let entry = &mut entries[idx];
        if !entry.get("config").is_some() {
            entry["config"] = serde_json::json!({});
        }
        if !entry["config"].get("mcpServers").is_some() {
            entry["config"]["mcpServers"] = serde_json::json!({});
        }
        if let Some(servers) = entry["config"]["mcpServers"].as_object_mut() {
            servers.insert(server.name, serde_json::Value::Object(server_obj));
        }
    } else {
        // 新增一个 MCP 插件 entry
        let mut servers_map = serde_json::Map::new();
        servers_map.insert(server.name, serde_json::Value::Object(server_obj));
        let new_entry = serde_json::json!({
            "id": "mcp",
            "package": MCP_CLIENT_PKG,
            "config": {
                "mcpServers": serde_json::Value::Object(servers_map)
            }
        });
        entries.push(new_entry);
    }

    let serialized = serde_yaml::to_string(&entries)
        .map_err(|e| format!("序列化 cordis.patch.yml 失败：{e}"))?;

    let tmp = profile_dir.join(format!("{PROFILE_PATCH_FILENAME}.tmp.{}", std::process::id()));
    std::fs::write(&tmp, serialized).map_err(|e| format!("写入临时 patch 失败：{e}"))?;
    std::fs::rename(&tmp, &patch_path).map_err(|e| format!("覆盖 patch 失败：{e}"))?;

    Ok(())
}

/// 删除指定 MCP 服务器配置
pub fn delete_mcp_server(
    home: &Path,
    profile: &str,
    server_name: &str,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let profile_dir = home.join("profiles").join(profile);
    let patch_path = profile_dir.join(PROFILE_PATCH_FILENAME);
    if !patch_path.is_file() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&patch_path)
        .map_err(|e| format!("读取 cordis.patch.yml 失败：{e}"))?;
    if content.trim().is_empty() {
        return Ok(());
    }

    let mut entries: Vec<serde_json::Value> = serde_yaml::from_str(&content)
        .map_err(|e| format!("解析 cordis.patch.yml 失败：{e}"))?;

    for entry in entries.iter_mut() {
        if entry.get("package").and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG) {
            if let Some(config) = entry.get_mut("config") {
                if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                    servers.remove(server_name);
                }
            }
        }
    }

    let serialized = serde_yaml::to_string(&entries)
        .map_err(|e| format!("序列化 cordis.patch.yml 失败：{e}"))?;

    let tmp = profile_dir.join(format!("{PROFILE_PATCH_FILENAME}.tmp.{}", std::process::id()));
    std::fs::write(&tmp, serialized).map_err(|e| format!("写入临时 patch 失败：{e}"))?;
    std::fs::rename(&tmp, &patch_path).map_err(|e| format!("覆盖 patch 失败：{e}"))?;

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
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
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
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
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
}
