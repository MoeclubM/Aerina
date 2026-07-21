use aerina_domain::{McpServer, McpToolCallResult, McpToolInfo, McpTransport};
use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static RPC_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct McpClient {
    client: Client,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn list_tools(&self, server: &McpServer) -> Result<Vec<McpToolInfo>> {
        let mut session = self.connect(server).await?;
        let result = session
            .request("tools/list", json!({}))
            .await
            .context("tools/list failed")?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(tools
            .into_iter()
            .filter_map(|tool| {
                let name = tool.get("name")?.as_str()?.to_string();
                let description = tool
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let input_schema = tool
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
                Some(McpToolInfo {
                    server_id: server.id.to_string(),
                    server_name: server.name.clone(),
                    name,
                    description,
                    input_schema,
                })
            })
            .collect())
    }

    pub async fn call_tool(
        &self,
        server: &McpServer,
        tool_name: &str,
        arguments: Value,
    ) -> Result<McpToolCallResult> {
        let mut session = self.connect(server).await?;
        let result = session
            .request(
                "tools/call",
                json!({
                    "name": tool_name,
                    "arguments": arguments,
                }),
            )
            .await
            .context("tools/call failed")?;
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(McpToolCallResult {
            content: result,
            is_error,
        })
    }

    pub async fn test_connection(&self, server: &McpServer) -> Result<String> {
        let tools = self.list_tools(server).await?;
        Ok(format!("ok: {} tools from {}", tools.len(), server.name))
    }

    async fn connect(&self, server: &McpServer) -> Result<McpSession> {
        match server.transport {
            McpTransport::StreamableHttp => self.connect_streamable_http(server).await,
            McpTransport::Sse => self.connect_sse(server).await,
        }
    }

    async fn connect_streamable_http(&self, server: &McpServer) -> Result<McpSession> {
        let mut session = McpSession {
            client: self.client.clone(),
            server: server.clone(),
            endpoint: server.url.clone(),
            session_id: None,
            mode: TransportMode::StreamableHttp,
            sse_post_url: None,
        };
        let init = session
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "aerina",
                        "version": "0.1.0"
                    }
                }),
            )
            .await
            .context("mcp initialize failed")?;
        let _ = init;
        session
            .notify("notifications/initialized", json!({}))
            .await?;
        Ok(session)
    }

    async fn connect_sse(&self, server: &McpServer) -> Result<McpSession> {
        // Classic SSE transport: GET SSE endpoint, then POST messages to endpoint from event.
        let mut req = self
            .client
            .get(&server.url)
            .header("Accept", "text/event-stream");
        for (k, v) in &server.headers {
            req = req.header(k, v);
        }
        let response = req.send().await?.error_for_status()?;
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut post_url: Option<String> = None;
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(10) {
            let Some(chunk) = byte_stream.next().await else {
                break;
            };
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(idx) = buffer.find('\n') {
                let line = buffer[..idx].trim().to_string();
                buffer = buffer[idx + 1..].to_string();
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if data.starts_with("http://") || data.starts_with("https://") {
                        post_url = Some(data.to_string());
                        break;
                    }
                    if let Ok(v) = serde_json::from_str::<Value>(data) {
                        if let Some(ep) = v.get("endpoint").and_then(|x| x.as_str()) {
                            post_url = Some(ep.to_string());
                            break;
                        }
                    }
                }
            }
            if post_url.is_some() {
                break;
            }
        }
        let post_url =
            post_url.ok_or_else(|| anyhow!("sse transport did not provide message endpoint"))?;
        let mut session = McpSession {
            client: self.client.clone(),
            server: server.clone(),
            endpoint: post_url.clone(),
            session_id: None,
            mode: TransportMode::Sse,
            sse_post_url: Some(post_url),
        };
        let _ = session
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "aerina",
                        "version": "0.1.0"
                    }
                }),
            )
            .await?;
        session
            .notify("notifications/initialized", json!({}))
            .await?;
        Ok(session)
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
enum TransportMode {
    StreamableHttp,
    Sse,
}

struct McpSession {
    client: Client,
    server: McpServer,
    endpoint: String,
    session_id: Option<String>,
    mode: TransportMode,
    sse_post_url: Option<String>,
}

impl McpSession {
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = RPC_ID.fetch_add(1, Ordering::Relaxed);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let response_value = self.post_rpc(payload, true).await?;
        if let Some(err) = response_value.get("error") {
            return Err(anyhow!("mcp error: {err}"));
        }
        response_value
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("mcp response missing result: {response_value}"))
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let _ = self.post_rpc(payload, false).await?;
        Ok(())
    }

    async fn post_rpc(&mut self, payload: Value, expect_body: bool) -> Result<Value> {
        let url = match self.mode {
            TransportMode::StreamableHttp => self.endpoint.clone(),
            TransportMode::Sse => self
                .sse_post_url
                .clone()
                .unwrap_or_else(|| self.endpoint.clone()),
        };
        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&payload);
        for (k, v) in &self.server.headers {
            req = req.header(k, v);
        }
        if let Some(session_id) = &self.session_id {
            req = req.header("Mcp-Session-Id", session_id);
        }
        let response = req.send().await?;
        if let Some(sid) = response.headers().get("mcp-session-id") {
            if let Ok(s) = sid.to_str() {
                self.session_id = Some(s.to_string());
            }
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("mcp http {status}: {text}"));
        }
        if !expect_body {
            return Ok(json!({}));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if content_type.contains("text/event-stream") {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(idx) = buffer.find('\n') {
                    let line = buffer[..idx].trim().to_string();
                    buffer = buffer[idx + 1..].to_string();
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" {
                            continue;
                        }
                        if let Ok(v) = serde_json::from_str::<Value>(data) {
                            if v.get("id").is_some()
                                || v.get("result").is_some()
                                || v.get("error").is_some()
                            {
                                return Ok(v);
                            }
                        }
                    }
                }
            }
            Err(anyhow!("sse rpc stream ended without result"))
        } else {
            let text = response.text().await?;
            if text.trim().is_empty() {
                return Ok(json!({}));
            }
            Ok(serde_json::from_str(&text).with_context(|| format!("invalid mcp json: {text}"))?)
        }
    }
}

pub fn tools_to_openai_definitions(tools: &[McpToolInfo]) -> Vec<aerina_domain::ToolDefinition> {
    tools
        .iter()
        .map(|t| aerina_domain::ToolDefinition {
            // Prefix with server short id to avoid collisions.
            name: sanitize_tool_name(&format!("{}_{}", short_id(&t.server_id), t.name)),
            description: Some(format!(
                "[{}] {}",
                t.server_name,
                t.description.clone().unwrap_or_default()
            )),
            parameters: t.input_schema.clone(),
        })
        .collect()
}

#[allow(dead_code)]
pub fn parse_prefixed_tool_name(name: &str) -> Option<(&str, &str)> {
    name.split_once('_')
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn list_tools_for_servers(
    client: &McpClient,
    servers: &[McpServer],
) -> Result<Vec<McpToolInfo>> {
    let mut all = Vec::new();
    for server in servers.iter().filter(|s| s.enabled) {
        match client.list_tools(server).await {
            Ok(mut tools) => all.append(&mut tools),
            Err(err) => {
                // Surface per-server failures to caller as empty with error string via anyhow aggregate?
                return Err(err.context(format!("server {}", server.name)));
            }
        }
    }
    Ok(all)
}

pub fn resolve_tool_target<'a>(
    tools: &'a [McpToolInfo],
    call_name: &str,
) -> Option<&'a McpToolInfo> {
    // Prefer exact prefixed match, then bare name.
    if let Some((prefix, raw)) = call_name.split_once('_') {
        if let Some(found) = tools
            .iter()
            .find(|t| short_id(&t.server_id) == prefix && t.name == raw)
        {
            return Some(found);
        }
    }
    tools.iter().find(|t| t.name == call_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aerina_domain::McpToolInfo;
    use serde_json::json;

    #[test]
    fn resolve_prefixed_tool_name() {
        let tools = vec![McpToolInfo {
            server_id: "01234567-aaaa-bbbb-cccc-ddddeeeeffff".into(),
            server_name: "demo".into(),
            name: "search".into(),
            description: Some("search things".into()),
            input_schema: json!({"type": "object"}),
        }];
        let defs = tools_to_openai_definitions(&tools);
        assert_eq!(defs.len(), 1);
        assert!(defs[0].name.starts_with("01234567_"));
        let found = resolve_tool_target(&tools, &defs[0].name).expect("prefixed");
        assert_eq!(found.name, "search");
        let bare = resolve_tool_target(&tools, "search").expect("bare");
        assert_eq!(bare.server_name, "demo");
    }
}
