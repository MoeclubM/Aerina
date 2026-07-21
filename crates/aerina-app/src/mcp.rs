use super::*;
use aerina_generation::ToolExecutor;
use aerina_mcp::{
    list_tools_for_servers, resolve_tool_target, tools_to_openai_definitions, McpClient,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct McpToolExecutor {
    client: McpClient,
    servers: Vec<McpServer>,
    tools: Vec<McpToolInfo>,
}

impl McpToolExecutor {
    pub fn new(servers: Vec<McpServer>, tools: Vec<McpToolInfo>) -> Self {
        Self {
            client: McpClient::new(),
            servers,
            tools,
        }
    }
}

#[async_trait]
impl ToolExecutor for McpToolExecutor {
    async fn execute(&self, name: &str, arguments: Value) -> Result<String> {
        let target = resolve_tool_target(&self.tools, name)
            .ok_or_else(|| anyhow!("unknown tool: {name}"))?;
        let server = self
            .servers
            .iter()
            .find(|s| s.id.to_string() == target.server_id)
            .ok_or_else(|| anyhow!("mcp server missing for tool {name}"))?;
        let result = self
            .client
            .call_tool(server, &target.name, arguments)
            .await?;
        if result.is_error {
            return Err(anyhow!("mcp tool returned error: {}", result.content));
        }
        Ok(result.content.to_string())
    }
}

impl AppState {
    pub async fn list_mcp_tools(&self) -> Result<Vec<McpToolInfo>> {
        let servers = self.list_mcp_servers().await?;
        let enabled: Vec<_> = servers.into_iter().filter(|s| s.enabled).collect();
        list_tools_for_servers(&McpClient::new(), &enabled).await
    }

    pub async fn test_mcp_server(&self, server_id: &str) -> Result<String> {
        let id = parse_entity_id(server_id)?;
        let servers = self.list_mcp_servers().await?;
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("mcp server not found"))?;
        McpClient::new().test_connection(&server).await
    }

    pub async fn call_mcp_tool(&self, request: McpToolCallRequest) -> Result<McpToolCallResult> {
        let id = parse_entity_id(&request.server_id)?;
        let servers = self.list_mcp_servers().await?;
        let server = servers
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("mcp server not found"))?;
        McpClient::new()
            .call_tool(&server, &request.tool_name, request.arguments)
            .await
    }

    pub(crate) async fn build_tool_executor(
        &self,
    ) -> Result<(Vec<ToolDefinition>, Option<Arc<dyn ToolExecutor>>)> {
        let servers = self.list_mcp_servers().await?;
        let enabled: Vec<_> = servers.into_iter().filter(|s| s.enabled).collect();
        if enabled.is_empty() {
            return Ok((Vec::new(), None));
        }
        let tools = list_tools_for_servers(&McpClient::new(), &enabled).await?;
        if tools.is_empty() {
            return Ok((Vec::new(), None));
        }
        let defs = tools_to_openai_definitions(&tools);
        let executor: Arc<dyn ToolExecutor> = Arc::new(McpToolExecutor::new(enabled, tools));
        Ok((defs, Some(executor)))
    }
}
