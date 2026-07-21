use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMode {
    Chat,
    Sbs,
    Arena,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArenaKind {
    Text,
    ImageGen,
    Vision,
    Mixed,
    Code,
    AgentTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTag {
    Text,
    Vision,
    ImageGeneration,
    Streaming,
    Reasoning,
    ToolCalling,
    AgentRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Pending,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteKind {
    Best,
    AllBad,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    GenerationStarted,
    GenerationCompleted,
    GenerationFailed,
    GenerationCancelled,
    ImageSaved,
    ArenaVoteCast,
    ArenaRevealed,
    CandidateCommitted,
    BranchForked,
    ModelSwitched,
    ProviderError,
    CostRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAiCompatible,
    OpenAi,
    OpenAiResponses,
    Anthropic,
    Gemini,
    Ollama,
    LmStudio,
    Custom,
}
