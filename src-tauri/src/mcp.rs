use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub server_name: String,
}

pub struct McpConnection {
    child: Child,
    pub server_name: String,
    request_id: u64,
}

impl McpConnection {
    pub fn connect(config: &McpServerConfig) -> Result<Self, String> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .envs(&config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to spawn MCP server '{}': {}", config.name, e))?;

        let mut conn = Self {
            child,
            server_name: config.name.clone(),
            request_id: 0,
        };

        // Initialize
        let _init_result = conn.send_request("initialize", serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "ubuntu-claude-desktop",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))?;

        // Send initialized notification
        conn.send_notification("notifications/initialized", serde_json::json!({}))?;

        Ok(conn)
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.request_id += 1;
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        let msg = serde_json::to_string(&request).map_err(|e| e.to_string())?;

        let stdin = self.child.stdin.as_mut()
            .ok_or("Failed to access stdin")?;
        writeln!(stdin, "{}", msg).map_err(|e| format!("Failed to write to MCP server: {}", e))?;
        stdin.flush().map_err(|e| e.to_string())?;

        // Read response line
        let stdout = self.child.stdout.as_mut()
            .ok_or("Failed to access stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| format!("Failed to read from MCP server: {}", e))?;

        let response: Value = serde_json::from_str(line.trim())
            .map_err(|e| format!("Invalid JSON from MCP server: {} (raw: {})", e, line.trim()))?;

        if let Some(error) = response.get("error") {
            return Err(format!("MCP error: {}", error));
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    fn send_notification(&mut self, method: &str, params: Value) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let msg = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
        let stdin = self.child.stdin.as_mut()
            .ok_or("Failed to access stdin")?;
        writeln!(stdin, "{}", msg).map_err(|e| format!("Failed to write notification: {}", e))?;
        stdin.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_tools(&mut self) -> Result<Vec<McpTool>, String> {
        let result = self.send_request("tools/list", serde_json::json!({}))?;

        let tools = result.get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        let mut mcp_tools = Vec::new();
        for tool in tools {
            mcp_tools.push(McpTool {
                name: tool.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                description: tool.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                input_schema: tool.get("inputSchema").cloned().unwrap_or(serde_json::json!({"type": "object"})),
                server_name: self.server_name.clone(),
            });
        }
        Ok(mcp_tools)
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        let result = self.send_request("tools/call", serde_json::json!({
            "name": name,
            "arguments": arguments
        }))?;
        Ok(result)
    }

    pub fn disconnect(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for McpConnection {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Connect to all configured MCP servers and collect their tools
pub fn collect_tools(configs: &[McpServerConfig]) -> (Vec<McpTool>, Vec<McpConnection>) {
    let mut all_tools = Vec::new();
    let mut connections = Vec::new();

    for config in configs {
        match McpConnection::connect(config) {
            Ok(mut conn) => {
                match conn.list_tools() {
                    Ok(tools) => {
                        all_tools.extend(tools);
                        connections.push(conn);
                    }
                    Err(e) => {
                        eprintln!("Failed to list tools from '{}': {}", config.name, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to MCP server '{}': {}", config.name, e);
            }
        }
    }

    (all_tools, connections)
}

/// Convert MCP tools to Anthropic API tool format
pub fn tools_to_api_format(tools: &[McpTool]) -> Vec<Value> {
    tools.iter().map(|t| {
        serde_json::json!({
            "name": t.name,
            "description": t.description,
            "input_schema": t.input_schema
        })
    }).collect()
}
