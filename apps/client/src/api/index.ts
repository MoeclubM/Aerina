import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ConversationMode = "chat" | "sbs" | "arena";
export type VoteKind = "best" | "all_bad" | "skip";
export type ArenaKind =
  | "text"
  | "image_gen"
  | "vision"
  | "mixed"
  | "code"
  | "agent_task";
export type McpTransport = "sse" | "streamable_http";

export type CapabilityTag =
  | "text"
  | "vision"
  | "image_generation"
  | "streaming"
  | "reasoning"
  | "tool_calling"
  | "agent_runtime";

export type ProviderKind =
  | "open_ai_compatible"
  | "open_ai"
  | "open_ai_responses"
  | "anthropic"
  | "gemini"
  | "ollama"
  | "lm_studio"
  | "custom";

/** UI-selectable provider endpoints only. */
export const PROVIDER_KIND_OPTIONS = [
  "open_ai_compatible",
  "open_ai_responses",
  "anthropic",
] as const satisfies readonly ProviderKind[];


export interface Profile {
  id: string;
  display_name: string;
  /** Relative path under media root; resolve via getProfileAvatarDataUrl */
  avatar_path?: string | null;
  /** Reserved for future remote login binding */
  auth_subject?: string | null;
  auth_provider?: string | null;
  created_at: string;
}

export interface SessionInfo {
  profile: Profile;
  profiles: Profile[];
}

export interface Conversation {
  id: string;
  title: string;
  mode: ConversationMode;
  active_branch_id?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ContentBlock {
  type: string;
  text?: string;
  media_id?: string;
  alt?: string;
  revised_prompt?: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  cached_prompt_tokens?: number;
  latency_ms?: number;
  ttft_ms?: number;
  cost_usd?: number;
}

export interface MessageView {
  message: {
    id: string;
    role: "system" | "user" | "assistant";
    created_at: string;
    candidate_id?: string | null;
    round_id?: string | null;
    parent_message_id?: string | null;
    branch_id?: string;
  };
  blocks: ContentBlock[];
}

export interface Branch {
  id: string;
  conversation_id: string;
  parent_branch_id?: string | null;
  fork_candidate_id?: string | null;
  head_message_id?: string | null;
  created_at: string;
}

export interface ConversationSettings {
  conversation_id: string;
  mode: ConversationMode;
  model_preset_ids: string[];
  candidate_pool?: string[];
  slot_count?: number;
  system_prompt?: string | null;
  temperature?: number | null;
  image_size?: string | null;
  arena_kind?: ArenaKind | null;
}

export interface CandidateInfo {
  id: string;
  round_id: string;
  slot_label: string;
  model_name: string;
  model_preset_id: string;
}

export interface RoundInfo {
  id: string;
  selected_candidate_id?: string | null;
}

export interface ConversationDetail {
  conversation: Conversation;
  settings: ConversationSettings;
  messages: MessageView[];
  branches: Branch[];
  rounds: RoundInfo[];
  candidates: CandidateInfo[];
  has_more: boolean;
}

export interface Provider {
  id: string;
  name: string;
  kind: ProviderKind;
  base_url: string;
  enabled: boolean;
}

export interface ModelPreset {
  id: string;
  provider_id: string;
  name: string;
  model_name: string;
  capabilities: CapabilityTag[];
  temperature?: number | null;
  enabled: boolean;
}

export interface ModelInfo {
  model_name: string;
  display_name: string;
  capabilities: CapabilityTag[];
  context_length?: number | null;
  family?: string | null;
}

export interface EloEntry {
  model_preset_id: string;
  arena_kind: string;
  category?: string | null;
  rating: number;
  games: number;
  wins: number;
}

export interface McpServer {
  id: string;
  name: string;
  transport: McpTransport;
  url: string;
  headers: Array<[string, string]>;
  enabled: boolean;
  created_at: string;
}

export interface McpToolInfo {
  server_id: string;
  server_name: string;
  name: string;
  description?: string | null;
  input_schema: Record<string, unknown>;
}

export interface McpToolCallResult {
  content: unknown;
  is_error: boolean;
}

export interface BackupInfo {
  name: string;
  path: string;
}

export interface RoundCandidate {
  id: string;
  slot_label: string;
  model_name?: string;
  model_preset_id?: string;
  status: string;
  anonymous: boolean;
  revealed: boolean;
}

export interface StatsSummary {
  request_count: number;
  completed_count: number;
  failed_count: number;
  cancelled_count: number;
  total_tokens: number;
  total_cost_usd: number;
  avg_latency_ms?: number | null;
  image_count: number;
  arena_votes: number;
  by_event_type: Record<string, number>;
}

export interface ActivityDay {
  date: string;
  count: number;
}

export interface ConversationExport {
  version: number;
  conversation: Conversation;
  settings: ConversationSettings;
  branches: Branch[];
  messages: MessageView[];
}

export interface GenerationEventPayload {
  conversationId: string;
  event:
    | { type: "stream_start"; candidate_id: string; slot_label: string }
    | { type: "text_delta"; candidate_id: string; delta: string }
    | { type: "thinking_delta"; candidate_id: string; delta: string }
    | { type: "usage"; candidate_id: string; usage: Record<string, unknown> }
    | { type: "done"; candidate_id: string }
    | { type: "error"; candidate_id: string; message: string }
    | { type: "image_ready"; candidate_id: string; image: Record<string, unknown> }
    | { type: "tool_calls"; candidate_id: string; calls: Array<Record<string, unknown>> };
}

export function errMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const anyErr = err as { message?: unknown };
    if (typeof anyErr.message === "string") return anyErr.message;
  }
  return String(err);
}

export const api = {

  sessionInfo: () => invoke<SessionInfo>("session_info"),
  createProfile: (request: { display_name: string }) =>
    invoke<SessionInfo>("create_profile", { request }),
  renameProfile: (profileId: string, displayName: string) =>
    invoke<Profile>("rename_profile", { profileId, displayName }),
  setProfileAvatar: (profileId: string, dataUrl: string) =>
    invoke<Profile>("set_profile_avatar", { profileId, dataUrl }),
  clearProfileAvatar: (profileId: string) =>
    invoke<Profile>("clear_profile_avatar", { profileId }),
  getProfileAvatarDataUrl: (profileId: string) =>
    invoke<string | null>("get_profile_avatar_data_url", { profileId }),
  switchProfile: (profileId: string) =>
    invoke<SessionInfo>("switch_profile", { profileId }),

  renameConversation: (conversationId: string, title: string) =>
    invoke<Conversation>("rename_conversation", { conversationId, title }),
  listConversations: () => invoke<Conversation[]>("list_conversations"),
  getConversation: (
    conversationId: string,
    opts?: { limit?: number; beforeMessageId?: string },
  ) =>
    invoke<ConversationDetail>("get_conversation", {
      conversationId,
      limit: opts?.limit ?? 100,
      beforeMessageId: opts?.beforeMessageId ?? null,
    }),
  createConversation: (request: {
    title?: string;
    mode: ConversationMode;
    model_preset_id?: string;
  }) => invoke<Conversation>("create_conversation", { request }),
  deleteConversation: (conversationId: string) =>
    invoke("delete_conversation", { conversationId }),
  listProviders: () => invoke<Provider[]>("list_providers"),
  upsertProvider: (request: {
    id?: string;
    name: string;
    kind: ProviderKind;
    base_url: string;
    api_key?: string;
    enabled?: boolean;
  }) => invoke<Provider>("upsert_provider", { request }),
  deleteProvider: (providerId: string) =>
    invoke("delete_provider", { providerId }),
  listModelPresets: () => invoke<ModelPreset[]>("list_model_presets"),
  listProviderModels: (providerId: string) =>
    invoke<ModelInfo[]>("list_provider_models", { providerId }),
  inferModelCompat: (modelName: string) =>
    invoke<ModelInfo>("infer_model_compat", { modelName }),
  upsertModelPreset: (request: {
    id?: string;
    provider_id: string;
    name: string;
    model_name: string;
    capabilities: CapabilityTag[];
    temperature?: number;
    system_prompt?: string;
    in_random_pool: boolean;
    enabled?: boolean;
  }) => invoke<ModelPreset>("upsert_model_preset", { request }),
  deleteModelPreset: (presetId: string) =>
    invoke("delete_model_preset", { presetId }),
  listMcpServers: () => invoke<McpServer[]>("list_mcp_servers"),
  upsertMcpServer: (request: {
    id?: string;
    name: string;
    transport: McpTransport;
    url: string;
    headers?: Array<[string, string]>;
    enabled?: boolean;
  }) => invoke<McpServer>("upsert_mcp_server", { request }),
  deleteMcpServer: (serverId: string) =>
    invoke("delete_mcp_server", { serverId }),
  listMcpTools: () => invoke<McpToolInfo[]>("list_mcp_tools"),
  testMcpServer: (serverId: string) =>
    invoke<string>("test_mcp_server", { serverId }),
  callMcpTool: (request: {
    server_id: string;
    tool_name: string;
    arguments: Record<string, unknown>;
  }) => invoke<McpToolCallResult>("call_mcp_tool", { request }),
  updateConversationModel: (conversationId: string, modelPresetId: string) =>
    invoke<ConversationSettings>("update_conversation_model", {
      conversationId,
      modelPresetId,
    }),
  updateConversationSettings: (
    conversationId: string,
    systemPrompt?: string | null,
    temperature?: number | null,
  ) =>
    invoke<ConversationSettings>("update_conversation_settings", {
      conversationId,
      systemPrompt,
      temperature,
    }),
  cancelGeneration: (conversationId: string) =>
    invoke("cancel_generation", { conversationId }),
  leaderboard: () => invoke<EloEntry[]>("leaderboard"),
  statsSummary: () => invoke<StatsSummary>("stats_summary"),
  activityHeatmap: (days?: number) =>
    invoke<ActivityDay[]>("activity_heatmap", { days: days ?? 365 }),
  setConversationModels: (
    conversationId: string,
    modelPresetIds: string[],
    mode: ConversationMode,
  ) =>
    invoke<ConversationSettings>("set_conversation_models", {
      conversationId,
      modelPresetIds,
      mode,
    }),
  commitCandidate: (conversationId: string, candidateId: string) =>
    invoke<ConversationDetail>("commit_candidate", {
      conversationId,
      candidateId,
    }),
  forkCandidate: (conversationId: string, candidateId: string) =>
    invoke<ConversationDetail>("fork_candidate", {
      conversationId,
      candidateId,
    }),
  switchBranch: (conversationId: string, branchId: string) =>
    invoke<ConversationDetail>("switch_branch", {
      conversationId,
      branchId,
    }),
  configureArena: (
    conversationId: string,
    pool: string[],
    slotCount: number,
    arenaKind: ArenaKind,
    category?: string | null,
  ) =>
    invoke<ConversationSettings>("configure_arena", {
      conversationId,
      pool,
      slotCount,
      arenaKind,
      category,
    }),
  listRoundCandidates: (roundId: string) =>
    invoke<RoundCandidate[]>("list_round_candidates", { roundId }),
  castArenaVote: (
    conversationId: string,
    roundId: string,
    voteKind: VoteKind,
    selectedCandidateId?: string | null,
  ) =>
    invoke<ConversationDetail>("cast_arena_vote", {
      conversationId,
      roundId,
      voteKind,
      selectedCandidateId,
    }),
  exportConversation: (conversationId: string) =>
    invoke<ConversationExport>("export_conversation", { conversationId }),
  importConversation: (payload: ConversationExport) =>
    invoke<Conversation>("import_conversation", { payload }),
  getMediaDataUrl: (mediaId: string) =>
    invoke<string>("get_media_data_url", { mediaId }),
  regenerate: (conversationId: string) =>
    invoke<ConversationDetail>("regenerate", { conversationId }),
  retryCandidate: (conversationId: string, candidateId: string) =>
    invoke<ConversationDetail>("retry_candidate", {
      conversationId,
      candidateId,
    }),
  createBackup: () => invoke<BackupInfo>("create_backup"),
  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  restoreBackup: (name: string) => invoke<string>("restore_backup", { name }),
  editUserMessage: (
    conversationId: string,
    messageId: string,
    content: string,
  ) =>
    invoke<ConversationDetail>("edit_user_message", {
      conversationId,
      messageId,
      content,
    }),
  sendMessage: (
    conversationId: string,
    content: string,
    options?: {
      image_data_urls?: string[];
      require_image?: boolean;
      image_size?: string;
    },
  ) =>
    invoke<ConversationDetail>("send_message", {
      request: {
        conversation_id: conversationId,
        content,
        image_data_urls: options?.image_data_urls,
        require_image: options?.require_image,
        image_size: options?.image_size,
      },
    }),
  listenGeneration: async (
    handler: (payload: GenerationEventPayload) => void,
  ): Promise<UnlistenFn> =>
    listen<GenerationEventPayload>("generation-event", (event) =>
      handler(event.payload),
    ),
};
