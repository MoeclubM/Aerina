<script setup lang="ts">
defineOptions({ name: "ChatPage" });

import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";

import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import MarkdownView from "../components/MarkdownView.vue";
import ConversationSidebar from "../components/ConversationSidebar.vue";
import {
  api,
  errMessage,
  type Conversation,
  type ConversationDetail,
  type MessageView,
  type ModelPreset,
  type SessionInfo,
} from "../api";
import { buildTimeline, imagesOf, textOf, thinkingMetaOf, thinkingOf, usageOf } from "../composables/useMessageTimeline";
import ThinkingBlock from "../components/ThinkingBlock.vue";
import { useStreamStore } from "../stores/stream";
import { useRoleStore } from "../stores/roles";

const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true });

const { t } = useI18n();
const router = useRouter();

const windowWidth = ref(window.innerWidth);
const handleResize = () => {
  windowWidth.value = window.innerWidth;
};
const mobile = computed(() => windowWidth.value < 680);
const stream = useStreamStore();
const roleStore = useRoleStore();

function formatUsageMeta(obj: {
  tokens?: number;
  promptTokens?: number;
  completionTokens?: number;
  cachedTokens?: number;
  latency?: number;
  ttft?: number;
}) {
  if (!obj) return [];
  const parts: Array<{ label: string; value: string }> = [];

  if (obj.promptTokens != null) {
    parts.push({ label: "输入", value: obj.promptTokens.toLocaleString() });
  }
  if (obj.completionTokens != null) {
    parts.push({ label: "输出", value: obj.completionTokens.toLocaleString() });
  }
  if (obj.cachedTokens != null) {
    parts.push({ label: "缓存", value: obj.cachedTokens.toLocaleString() });
  }
  if (obj.ttft != null) {
    parts.push({ label: "首字", value: `${obj.ttft}ms` });
  }
  if (obj.latency != null) {
    const sec = obj.latency < 1000 ? `${obj.latency}ms` : `${(obj.latency / 1000).toFixed(1)}s`;
    parts.push({ label: "总用时", value: sec });
  }

  // Fallback if detailed tokens are not present
  if (!parts.length && obj.tokens != null) {
    parts.push({ label: "Token", value: `${obj.tokens.toLocaleString()} tok` });
  }

  return parts;
}

function hasUsageInfo(obj: any) {
  if (!obj) return false;
  return obj.tokens != null || obj.promptTokens != null || obj.latency != null || obj.ttft != null;
}

const session = ref<SessionInfo | null>(null);
const mobileCandidateIndex = ref(0);
const titleDraft = ref("");
const temperature = ref(0.7);
const selectedRoleId = ref(roleStore.defaultRoleId);
const editingTitle = ref(false);

const conversations = shallowRef<Conversation[]>([]);
const presets = shallowRef<ModelPreset[]>([]);
const detail = shallowRef<ConversationDetail | null>(null);
const selectedId = ref<string | null>(null);
const selectedModels = ref<string[]>([]);
const draft = ref("");
const systemPrompt = ref("");
const editingId = ref<string | null>(null);
const editDraft = ref("");
const attachments = ref<string[]>([]);
const addMenu = ref(false);
const dragOver = ref(false);
const pendingUser = ref<{ text: string; imageUrls: string[] } | null>(null);
const showList = ref(true);
const error = ref<string | null>(null);
const imageUrls = ref<Record<string, string>>({});
const convFilter = ref("");
const pinBottom = ref(true);
const settingsOpen = ref(false);
const modelMenu = ref(false);
const modeMenu = ref(false);
const parentRef = ref<HTMLElement | null>(null);
const importInput = ref<HTMLInputElement | null>(null);
let unlisten: (() => void) | undefined;
let mediaQueue: string[] = [];
let mediaLoading = false;
const PAGE_SIZE = 100;
const MAX_MESSAGES = 300;
const loadingOlder = ref(false);
const loadingNewer = ref(false);
const hasMoreOlder = ref(false);
const hasMoreNewer = ref(false);

type CandView = {
  messageId: string;
  candidateId?: string | null;
  text: string;
  thinking: string;
  cacheKey: string;
  label: string;
  letter: string;
  modelName?: string;
  selected: boolean;
  thinkingTokens?: number;
  thinkingDurationMs?: number;
};

type ViewRow =
  | { kind: "user"; key: string; messageId: string; text: string; cacheKey: string; imageIds: string[] }
  | { kind: "pending_user"; key: string; text: string; imageUrls: string[] }
  | { kind: "assistant"; key: string; messageId: string; text: string; thinking: string; cacheKey: string; imageIds: string[]; tokens?: number; promptTokens?: number; completionTokens?: number; cachedTokens?: number; latency?: number; ttft?: number; thinkingTokens?: number; thinkingDurationMs?: number }
  | { kind: "round"; key: string; selectedCandidateId?: string | null; candidates: CandView[] }
  | { kind: "streaming"; key: string }
  | { kind: "error"; key: string; message: string };

function contentKey(id: string, text: string) {
  return id + ":" + String(text.length) + ":" + text.slice(0, 24) + ":" + text.slice(-24);
}

const selectedByRound = computed(() => {
  const map: Record<string, string | null | undefined> = {};
  for (const r of detail.value?.rounds ?? []) {
    map[r.id] = r.selected_candidate_id ?? null;
  }
  return map;
});

const candidateMeta = computed(() => {
  const map = new Map<string, { modelName: string; slot: string }>();
  for (const c of detail.value?.candidates ?? []) {
    map.set(c.id, { modelName: c.model_name, slot: c.slot_label });
  }
  return map;
});

const rows = computed<ViewRow[]>(() => {
  const list: ViewRow[] = [];
  for (const item of buildTimeline(detail.value?.messages ?? [], selectedByRound.value)) {
    if (item.kind === "user") {
      const text = textOf(item.message);
      list.push({
        kind: "user",
        key: item.key,
        messageId: item.message.message.id,
        text,
        cacheKey: contentKey(item.message.message.id, text),
        imageIds: imagesOf(item.message).map((i) => i.media_id!).filter(Boolean),
      });
    } else if (item.kind === "assistant") {
      const text = textOf(item.message);
      const thinking = thinkingOf(item.message);
      const usage = usageOf(item.message);
      const tmeta = thinkingMetaOf(item.message);
      list.push({
        kind: "assistant",
        key: item.key,
        messageId: item.message.message.id,
        text,
        thinking,
        cacheKey: contentKey(item.message.message.id, text + "\0" + thinking),
        imageIds: imagesOf(item.message).map((i) => i.media_id!).filter(Boolean),
        tokens: usage?.total_tokens,
        promptTokens: usage?.prompt_tokens,
        completionTokens: usage?.completion_tokens,
        cachedTokens: usage?.cached_prompt_tokens,
        latency: usage?.latency_ms,
        ttft: usage?.ttft_ms,
        thinkingTokens: tmeta.tokens,
        thinkingDurationMs: tmeta.durationMs,
      });
    } else {
      const selectedIdCand = item.selectedCandidateId ?? null;
      list.push({
        kind: "round",
        key: item.key,
        selectedCandidateId: selectedIdCand,
        candidates: item.messages.map((m, idx) => {
          const text = textOf(m);
          const thinking = thinkingOf(m);
          const tmeta = thinkingMetaOf(m);
          const letter = String.fromCharCode(65 + idx);
          const cid = m.message.candidate_id ?? "";
          const meta = cid ? candidateMeta.value.get(cid) : undefined;
          const label = meta?.modelName || `${t("chat.candidate")} ${letter}`;
          return {
            messageId: m.message.id,
            candidateId: m.message.candidate_id,
            text,
            thinking,
            cacheKey: contentKey(m.message.id, text + "\0" + thinking),
            label,
            letter: meta?.slot || letter,
            modelName: meta?.modelName,
            selected: !!selectedIdCand && cid === selectedIdCand,
            thinkingTokens: tmeta.tokens,
            thinkingDurationMs: tmeta.durationMs,
          };
        }),
      });
    }
  }
  if (pendingUser.value && stream.isStreaming && stream.conversationId === selectedId.value) {
    list.push({
      kind: "pending_user",
      key: "pending-user",
      text: pendingUser.value.text,
      imageUrls: pendingUser.value.imageUrls,
    });
  }
  if (stream.isStreaming && stream.conversationId === selectedId.value) {
    list.push({ kind: "streaming", key: "streaming" });
  }
  return list;
});

const bannerError = computed(() => stream.error || error.value);
const showWelcome = computed(() => !selectedId.value);
const showThreadEmpty = computed(
  () =>
    !!selectedId.value &&
    !(detail.value?.messages?.length) &&
    !pendingUser.value &&
    !(stream.isStreaming && stream.conversationId === selectedId.value),
);


const multiModel = computed(() => selectedModels.value.length > 1);
const hasConversation = computed(() => !!selectedId.value);
const canSend = computed(
  () =>
    !!selectedId.value &&
    selectedModels.value.length > 0 &&
    !stream.isStreaming &&
    (!!draft.value.trim() || attachments.value.length > 0),
);

const modelButtonLabel = computed(() => {
  if (!selectedModels.value.length) return t("chat.selectModel");
  if (selectedModels.value.length === 1) {
    const p = presets.value.find((x) => x.id === selectedModels.value[0]);
    return p ? p.name : t("chat.singleModel");
  }
  return `${selectedModels.value.length}× ${t("chat.models")}`;
});

const selectedPresetNames = computed(() =>
  selectedModels.value
    .map((id) => presets.value.find((p) => p.id === id)?.name)
    .filter(Boolean)
    .join(", "),
);

const branches = computed(() => detail.value?.branches ?? []);






const virtualizer = useVirtualizer(
  computed(() => ({
    count: rows.value.length,
    getScrollElement: () => parentRef.value,
    estimateSize: (index: number) => {
      const row = rows.value[index];
      if (!row) return 120;
      if (row.kind === "round") return 320;
      if (row.kind === "streaming" && stream.candidates.length > 1) return 320;
      if (row.kind === "user" || row.kind === "pending_user") return row.kind === "pending_user" && (row as any).imageUrls?.length ? 180 : 88;
      if (row.kind === "assistant") {
        const lines = Math.ceil(row.text.length / 90);
        return Math.min(480, 72 + lines * 22);
      }
      return 140;
    },
    overscan: 8,
    getItemKey: (index: number) => rows.value[index]?.key ?? index,
  })),
);

const virtualItems = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());

function measureRef(el: Element | { $el?: Element } | null) {
  const node = el && "$el" in el ? el.$el ?? null : el;
  if (node instanceof Element) virtualizer.value.measureElement(node);
}

function rowAt(index: number) {
  return rows.value[index];
}

function capMessages(messages: MessageView[], preferOlder: boolean): MessageView[] {
  if (messages.length <= MAX_MESSAGES) return messages;
  return preferOlder ? messages.slice(0, MAX_MESSAGES) : messages.slice(messages.length - MAX_MESSAGES);
}

async function ensureMedia(ids: string[]) {
  if (!props.active) return;
  for (const id of ids) {
    if (!imageUrls.value[id] && !mediaQueue.includes(id)) mediaQueue.push(id);
  }
  if (mediaLoading) return;
  mediaLoading = true;
  while (mediaQueue.length) {
    const id = mediaQueue.shift()!;
    if (imageUrls.value[id]) continue;
    try {
      const url = await api.getMediaDataUrl(id);
      imageUrls.value = { ...imageUrls.value, [id]: url };
    } catch {
      /* empty thumb */
    }
  }
  mediaLoading = false;
}

function collectMedia(list: MessageView[]) {
  const ids: string[] = [];
  for (const m of list) {
    for (const img of imagesOf(m)) {
      if (img.media_id) ids.push(img.media_id);
    }
  }
  if (ids.length) void ensureMedia(ids);
}

async function loadSession() {
  session.value = await api.sessionInfo();
}

watch(
  () => props.active,
  (active) => {
    if (active) void loadSession();
  },
);

async function onSwitchBranch(branchId: string) {
  if (!selectedId.value) return;
  if (branchId === detail.value?.conversation.active_branch_id) return;
  error.value = null;
  try {
    detail.value = await api.switchBranch(selectedId.value, branchId);
  } catch (e) {
    error.value = errMessage(e);
  }
}

async function onSessionChanged() {
  session.value = await api.sessionInfo();
  selectedId.value = null;
  detail.value = null;
  presets.value = await api.listModelPresets();
  await refreshConversations();
  await refreshDetail();
}

async function saveTitle() {
  if (!selectedId.value) return;
  const title = titleDraft.value.trim();
  if (!title) return;
  const conversation = await api.renameConversation(selectedId.value, title);
  if (detail.value) {
    detail.value = {
      ...detail.value,
      conversation: { ...detail.value.conversation, title: conversation.title },
    };
  }
  editingTitle.value = false;
  await refreshConversations();
}

async function refreshConversations() {
  conversations.value = await api.listConversations();
  if (!selectedId.value && conversations.value.length) selectedId.value = conversations.value[0].id;
}

async function refreshDetail() {
  if (!selectedId.value) {
    detail.value = null;
    hasMoreOlder.value = false;
    hasMoreNewer.value = false;
    pendingUser.value = null;
    return;
  }
  const page = await api.getConversation(selectedId.value, { limit: PAGE_SIZE });
  detail.value = page;
  if (!stream.isStreaming) pendingUser.value = null;
  hasMoreOlder.value = page.has_more;
  hasMoreNewer.value = false;
  systemPrompt.value = page.settings.system_prompt ?? "";
  temperature.value = page.settings.temperature ?? 0.7;
  selectedRoleId.value = roleStore.convRoles[page.conversation.id] || roleStore.defaultRoleId;
  titleDraft.value = page.conversation.title;
  selectedModels.value = [...(page.settings.model_preset_ids ?? [])];
  mobileCandidateIndex.value = 0;
  collectMedia(page.messages);
  await nextTick();
  if (pinBottom.value) scrollToBottom();
  virtualizer.value.measure();
}

async function loadOlderMessages() {
  if (!selectedId.value || !detail.value || !hasMoreOlder.value || loadingOlder.value || loadingNewer.value) return;
  const oldest = detail.value.messages[0];
  if (!oldest) return;
  loadingOlder.value = true;
  const el = parentRef.value;
  const prevHeight = el?.scrollHeight ?? 0;
  const prevTop = el?.scrollTop ?? 0;
  try {
    const page = await api.getConversation(selectedId.value, {
      limit: PAGE_SIZE,
      beforeMessageId: oldest.message.id,
    });
    if (!page.messages.length) {
      hasMoreOlder.value = false;
      return;
    }
    let messages = [...page.messages, ...detail.value.messages];
    if (messages.length > MAX_MESSAGES) {
      messages = capMessages(messages, true);
      hasMoreNewer.value = true;
    }
    detail.value = {
      ...detail.value,
      messages,
      has_more: page.has_more,
    };
    hasMoreOlder.value = page.has_more;
    collectMedia(page.messages);
    await nextTick();
    if (el) el.scrollTop = prevTop + (el.scrollHeight - prevHeight);
  } finally {
    loadingOlder.value = false;
  }
}

async function loadNewerTail() {
  if (!selectedId.value || !hasMoreNewer.value || loadingNewer.value || loadingOlder.value) return;
  loadingNewer.value = true;
  try {
    const page = await api.getConversation(selectedId.value, { limit: PAGE_SIZE });
    detail.value = {
      conversation: page.conversation,
      settings: page.settings,
      branches: page.branches,
      messages: page.messages,
      rounds: page.rounds ?? [],
      candidates: page.candidates ?? [],
      has_more: page.has_more,
    };
    hasMoreOlder.value = page.has_more;
    hasMoreNewer.value = false;
    collectMedia(page.messages);
    await nextTick();
    scrollToBottom();
    virtualizer.value.measure();
  } finally {
    loadingNewer.value = false;
  }
}

function scrollToBottom() {
  const el = parentRef.value;
  if (el) el.scrollTop = el.scrollHeight;
}

function onScroll() {
  const el = parentRef.value;
  if (!el || !props.active) return;
  const distBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
  pinBottom.value = distBottom < 96 && !hasMoreNewer.value;
  if (el.scrollTop < 120) void loadOlderMessages();
  if (hasMoreNewer.value && distBottom < 120) void loadNewerTail();
}


const OPEN_TABS_KEY = "aerina.openChatTabs";

function loadOpenTabs(): string[] {
  try {
    const raw = localStorage.getItem(OPEN_TABS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

const openTabIds = ref<string[]>(loadOpenTabs());

function persistOpenTabs() {
  localStorage.setItem(OPEN_TABS_KEY, JSON.stringify(openTabIds.value));
}

function openTab(id: string, activate = true) {
  if (!id) return;
  if (!openTabIds.value.includes(id)) {
    openTabIds.value = [...openTabIds.value, id];
    persistOpenTabs();
  }
  if (activate) {
    selectedId.value = id;
    if (mobile.value) showList.value = false;
  }
}

function closeTab(id: string) {
  const idx = openTabIds.value.indexOf(id);
  if (idx < 0) return;
  const next = openTabIds.value.filter((x) => x !== id);
  openTabIds.value = next;
  persistOpenTabs();
  if (selectedId.value === id) {
    const fallback = next[Math.max(0, idx - 1)] ?? next[0] ?? null;
    selectedId.value = fallback;
    if (!fallback && mobile.value) showList.value = true;
  }
}

function closeOtherTabs(id: string) {
  openTabIds.value = id ? [id] : [];
  persistOpenTabs();
  if (id) selectedId.value = id;
}


watch(conversations, (list) => {
  const ids = new Set(list.map((c) => c.id));
  const next = openTabIds.value.filter((id) => ids.has(id));
  if (next.length !== openTabIds.value.length) {
    openTabIds.value = next;
    persistOpenTabs();
  }
  if (selectedId.value && !ids.has(selectedId.value)) {
    selectedId.value = next[next.length - 1] ?? null;
  }
});

const openTabs = computed(() => {
  const byId = new Map(conversations.value.map((c) => [c.id, c]));
  return openTabIds.value.map((id) => {
    const c = byId.get(id);
    return { id, title: c?.title || t("chat.select") };
  });
});

async function createConversation() {
  error.value = null;
  if (!presets.value.length) {
    error.value = t("chat.noModelsHint");
    return;
  }
  const defRole = roleStore.defaultRole;
  const presetId = selectedModels.value[0] ?? presets.value[0]?.id;
  const conversation = await api.createConversation({
    title: defRole.name,
    mode: selectedModels.value.length > 1 ? "sbs" : "chat",
    model_preset_id: presetId,
  });
  roleStore.setConversationRole(conversation.id, defRole.id);
  await api.updateConversationSettings(
    conversation.id,
    defRole.systemPrompt || null,
    defRole.temperature,
  );
  openTab(conversation.id);
  if (selectedModels.value.length > 1) {
    await api.setConversationModels(conversation.id, selectedModels.value, "sbs");
  }
  await refreshConversations();
  await refreshDetail();
  if (mobile.value) showList.value = false;
}

async function createConversationWithRole(roleId: string) {
  error.value = null;
  if (!presets.value.length) {
    error.value = t("chat.noModelsHint");
    return;
  }
  const role = roleStore.roles.find((r) => r.id === roleId) || roleStore.defaultRole;
  const presetId = selectedModels.value[0] ?? presets.value[0]?.id;
  const conversation = await api.createConversation({
    title: role.name,
    mode: selectedModels.value.length > 1 ? "sbs" : "chat",
    model_preset_id: presetId,
  });
  roleStore.setConversationRole(conversation.id, role.id);
  await api.updateConversationSettings(
    conversation.id,
    role.systemPrompt || null,
    role.temperature,
  );
  openTab(conversation.id);
  if (selectedModels.value.length > 1) {
    await api.setConversationModels(conversation.id, selectedModels.value, "sbs");
  }
  await refreshConversations();
  await refreshDetail();
  if (mobile.value) showList.value = false;
}

async function deleteConversation(id: string) {
  await api.deleteConversation(id);
  roleStore.removeConversationRole(id);
  closeTab(id);
  await refreshConversations();
  await refreshDetail();
}

async function applyModels() {
  if (!selectedId.value || !selectedModels.value.length) return;
  const mode = selectedModels.value.length > 1 ? "sbs" : "chat";
  await api.setConversationModels(selectedId.value, selectedModels.value, mode);
  await refreshDetail();
}

function toggleModel(id: string) {
  const set = new Set(selectedModels.value);
  if (set.has(id)) {
    if (set.size === 1) return;
    set.delete(id);
  } else set.add(id);
  selectedModels.value = [...set];
  void applyModels();
}

function applyConversationRole(roleId: string) {
  const role = roleStore.roles.find((item) => item.id === roleId);
  if (!role) return;
  selectedRoleId.value = role.id;
  systemPrompt.value = role.systemPrompt;
  temperature.value = role.temperature;
}

async function saveSettings() {
  if (!selectedId.value) return;
  if (titleDraft.value.trim() && titleDraft.value.trim() !== detail.value?.conversation.title) {
    await api.renameConversation(selectedId.value, titleDraft.value.trim());
  }
  roleStore.setConversationRole(selectedId.value, selectedRoleId.value);
  await api.updateConversationSettings(
    selectedId.value,
    systemPrompt.value || null,
    temperature.value,
  );
  settingsOpen.value = false;
  await refreshDetail();
  await refreshConversations();
}

function autoGrowComposer(e?: Event) {
  const el = (e?.target as HTMLTextAreaElement | undefined) ?? (document.querySelector(".composer-textarea") as HTMLTextAreaElement | null);
  if (!el) return;
  el.style.height = "auto";
  el.style.height = Math.min(el.scrollHeight, 180) + "px";
}

async function send() {
  if (!canSend.value || !selectedId.value) return;
  error.value = null;
  pinBottom.value = true;
  hasMoreNewer.value = false;
  const content = draft.value.trim();
  const imgs = [...attachments.value];
  draft.value = "";
  requestAnimationFrame(() => autoGrowComposer());
  attachments.value = [];
  pendingUser.value = { text: content, imageUrls: imgs };
  stream.begin(selectedId.value);
  try {
    await applyModels();
    await api.sendMessage(selectedId.value, content, {
      image_data_urls: imgs.length ? imgs : undefined,
    });
    stream.finish();
    pendingUser.value = null;
    await refreshConversations();
    await refreshDetail();
  } catch (e) {
    stream.fail(errMessage(e));
    error.value = errMessage(e);
    // keep pendingUser so the failed send is still visible; user can edit/retry context via new message
  }
}

async function regenerate() {
  if (!selectedId.value) return;
  pinBottom.value = true;
  stream.begin(selectedId.value);
  try {
    await api.regenerate(selectedId.value);
    stream.finish();
    await refreshDetail();
  } catch (e) {
    stream.fail(errMessage(e));
  }
}

async function saveEdit(messageId: string) {
  if (!selectedId.value) return;
  pinBottom.value = true;
  stream.begin(selectedId.value);
  try {
    await api.editUserMessage(selectedId.value, messageId, editDraft.value);
    stream.finish();
    editingId.value = null;
    await refreshDetail();
  } catch (e) {
    stream.fail(errMessage(e));
  }
}

async function stopGeneration() {
  if (!selectedId.value) return;
  await api.cancelGeneration(selectedId.value);
  stream.finish();
  error.value = null;
}

async function actCandidate(kind: "commit" | "fork" | "retry", candidateId: string) {
  if (!selectedId.value) return;
  error.value = null;
  try {
    if (kind === "commit") {
      await api.commitCandidate(selectedId.value, candidateId);
      await refreshConversations();
    }
    if (kind === "fork") await api.forkCandidate(selectedId.value, candidateId);
    if (kind === "retry") {
      stream.begin(selectedId.value);
      await api.retryCandidate(selectedId.value, candidateId);
      stream.finish();
    }
    await refreshDetail();
  } catch (e) {
    stream.fail(errMessage(e));
    error.value = errMessage(e);
  }
}

function pickAttachments() {
  addMenu.value = false;
  const input = document.createElement("input");
  input.type = "file";
  input.accept = "image/*";
  input.multiple = true;
  input.className = "sr-only-input";
  input.style.cssText = "position:fixed;left:-9999px;top:0;width:1px;height:1px;opacity:0;pointer-events:none;";
  input.onchange = () => {
    void onPickFiles(input.files);
    input.remove();
  };
  document.body.appendChild(input);
  input.click();
}

function openConversationSettings() {
  addMenu.value = false;
  if (selectedId.value) {
    selectedRoleId.value = roleStore.convRoles[selectedId.value] || roleStore.defaultRoleId;
  }
  settingsOpen.value = true;
}

function openMcpSettings() {
  addMenu.value = false;
  void router.push("/settings/mcp");
}

function openProviderSettings() {
  addMenu.value = false;
  void router.push("/settings/providers");
}

function openAgentEntry() {
  modeMenu.value = false;
  window.dispatchEvent(new CustomEvent("aerina:open-agent-entry"));
}

async function onPickFiles(files: FileList | File[] | null) {
  if (!files) return;
  const list = Array.from(files as FileList | File[]);
  const images = list.filter((f) => f.type.startsWith("image/"));
  const skipped = list.length - images.length;
  if (skipped > 0) {
    error.value = t("chat.onlyImages", { n: skipped });
  }
  for (const file of images) {
    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
    attachments.value.push(dataUrl);
  }
}

function onComposerDragOver(e: DragEvent) {
  if (!hasConversation.value || stream.isStreaming) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  dragOver.value = true;
}

function onComposerDragLeave(e: DragEvent) {
  const el = e.currentTarget as HTMLElement;
  const related = e.relatedTarget as Node | null;
  if (related && el.contains(related)) return;
  dragOver.value = false;
}

function onComposerDrop(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;
  if (!hasConversation.value || stream.isStreaming) return;
  const files = e.dataTransfer?.files;
  if (files?.length) void onPickFiles(files);
}

async function importConversation(files: FileList | null) {
  if (!files?.length) return;
  const conversation = await api.importConversation(JSON.parse(await files[0].text()));
  await refreshConversations();
  openTab(conversation.id);
}

async function exportConversation(id: string) {
  const payload = await api.exportConversation(id);
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `aerina-${id.slice(0, 8)}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function gridClass(count: number) {
  if (count <= 1) return "";
  if (count === 2) return "cols-2";
  if (count === 3) return "cols-3";
  return "cols-4";
}

watch(selectedId, async (id) => {
  if (id && !openTabIds.value.includes(id)) {
    openTabIds.value = [...openTabIds.value, id];
    persistOpenTabs();
  }
  attachments.value = [];
  pendingUser.value = null;
  error.value = null;
  editingTitle.value = false;
  editingId.value = null;
  mobileCandidateIndex.value = 0;
  pinBottom.value = true;
  if (stream.conversationId && stream.conversationId !== selectedId.value) {
    stream.reset();
  }
  await refreshDetail();
});

watch(
  () => stream.combinedText,
  async () => {
    if (!props.active || !pinBottom.value) return;
    await nextTick();
    scrollToBottom();
  },
);

watch(
  () => props.active,
  async (active) => {
    if (!active) return;
    await nextTick();
    virtualizer.value.measure();
    if (pinBottom.value) scrollToBottom();
  },
);

async function setupListener() {
  if (unlisten) return;
  unlisten = await api.listenGeneration((payload) => {
    if (payload.conversationId !== stream.conversationId) return;
    const event = payload.event;
    if (event.type === "stream_start") stream.streamStart(event.candidate_id, event.slot_label);
    else if (event.type === "text_delta") stream.appendDelta(event.candidate_id, event.delta);
    else if (event.type === "thinking_delta") stream.appendThinking(event.candidate_id, event.delta);
    else if (event.type === "usage") stream.applyUsage(event.candidate_id, event.usage);
    else if (event.type === "done") stream.markDone(event.candidate_id);
    else if (event.type === "error") stream.markError(event.candidate_id, event.message);
  });
}


onMounted(async () => {
  window.addEventListener("aerina:session-changed", onSessionChanged);
  window.addEventListener("resize", handleResize);
  await loadSession();
  presets.value = await api.listModelPresets();
  await refreshConversations();
  await refreshDetail();
  await setupListener();
});


watch(
  () => [props.active, mobile.value, selectedId.value, showList.value],
  () => {
    const inSubpage = Boolean(props.active && mobile.value && selectedId.value && !showList.value);
    window.dispatchEvent(new CustomEvent("aerina:mobile-subpage-changed", { detail: { active: inSubpage } }));
  },
  { immediate: true },
);


void bannerError;
void showThreadEmpty;
void totalSize;
void measureRef;
void closeOtherTabs;
void openTabs;
void regenerate;
void saveEdit;
void pickAttachments;
void openConversationSettings;
void openMcpSettings;
void openProviderSettings;
void onComposerDragOver;
void onComposerDragLeave;
void onComposerDrop;

onUnmounted(() => {
  window.removeEventListener("aerina:session-changed", onSessionChanged);
  window.removeEventListener("resize", handleResize);
  unlisten?.();
  unlisten = undefined;
});
</script>

<template>
  <div class="chat-root" :class="{ mobile, 'sidebar-hidden': mobile && !showList && selectedId }">
    <!-- Desktop Permanent Unified Glass Sidebar -->
    <ConversationSidebar
      v-if="!mobile"
      :conversations="conversations"
      :selected-id="selectedId"
      :filter="convFilter"
      :new-label="t('chat.new')"
      :search-label="t('chat.search')"
      :multi-label="t('chat.multiModel')"
      :single-label="t('chat.singleModel')"
      :today-label="t('chat.today')"
      :yesterday-label="t('chat.yesterday')"
      :earlier-label="t('chat.earlier')"
      @update:filter="convFilter = $event"
      @select="openTab($event)"
      @create-with-role="createConversationWithRole"
      @export="exportConversation"
      @remove="deleteConversation"
    />

    <!-- Mobile View 1: List View -->
    <div v-else v-show="showList || !selectedId" class="mobile-list-view">
      <ConversationSidebar
        :mobile="true"
        :conversations="conversations"
        :selected-id="selectedId"
        :filter="convFilter"
        :new-label="t('chat.new')"
        :search-label="t('chat.search')"
        :multi-label="t('chat.multiModel')"
        :single-label="t('chat.singleModel')"
        :today-label="t('chat.today')"
        :yesterday-label="t('chat.yesterday')"
        :earlier-label="t('chat.earlier')"
        @update:filter="convFilter = $event"
        @select="openTab($event)"
        @create-with-role="createConversationWithRole($event); showList = false"
        @export="exportConversation"
        @remove="deleteConversation"
      />
    </div>

    <input
      ref="importInput"
      type="file"
      accept="application/json"
      class="sr-only-input"
      style="position:fixed;left:-9999px;top:0;width:1px;height:1px;opacity:0;pointer-events:none;"
      tabindex="-1"
      aria-hidden="true"
      @change="importConversation(($event.target as HTMLInputElement).files); ($event.target as HTMLInputElement).value = ''"
    />

    <!-- Main Chat Content (Desktop or Mobile Dedicated Sub-Page) -->
    <section v-show="!mobile || (mobile && !showList && selectedId)" class="chat-main">
      <div v-if="openTabs.length" class="chat-tabs-bar" data-tauri-drag-region>
        <div class="chat-tabs-scroll" role="tablist" :aria-label="t('chat.tabs')">
          <div
            v-for="tab in openTabs"
            :key="tab.id"
            class="chat-tab"
            role="tab"
            :aria-selected="selectedId === tab.id"
            :class="{ active: selectedId === tab.id }"
            :title="tab.title"
            @click="openTab(tab.id)"
            @auxclick.middle.prevent="closeTab(tab.id)"
          >
            <span class="chat-tab-title">{{ tab.title }}</span>
            <button
              type="button"
              class="chat-tab-close"
              :title="t('chat.closeTab')"
              :aria-label="t('chat.closeTab')"
              @click.stop="closeTab(tab.id)"
            >
              <v-icon icon="mdi-close" size="12" />
            </button>
          </div>
        </div>
        <div class="chat-tabs-actions">
          <button
            type="button"
            class="chat-tabs-action"
            :title="t('chat.new')"
            :aria-label="t('chat.new')"
            @click="createConversation"
          >
            <v-icon icon="mdi-plus" size="16" />
          </button>
          <button
            v-if="selectedId && openTabs.length > 1"
            type="button"
            class="chat-tabs-action"
            :title="t('chat.closeOtherTabs')"
            :aria-label="t('chat.closeOtherTabs')"
            @click="closeOtherTabs(selectedId)"
          >
            <v-icon icon="mdi-close-box-multiple-outline" size="16" />
          </button>
        </div>
      </div>
      <header v-if="hasConversation" class="chat-toolbar">
        <v-btn
          v-if="mobile"
          icon="mdi-chevron-left"
          variant="text"
          density="comfortable"
          class="me-1 chat-back-btn"
          :title="t('chat.backToList')"
          :aria-label="t('chat.backToList')"
          @click="showList = true"
        />
        <div class="chat-title">
          <div v-if="editingTitle && selectedId" class="chat-title-edit d-flex align-center ga-1">
            <v-text-field
              v-model="titleDraft"
              density="compact"
              hide-details
              variant="solo-filled"
              flat
              class="title-edit-field"
              :placeholder="t('chat.titlePlaceholder')"
              @keydown.enter.prevent="saveTitle"
              @keydown.esc.prevent="editingTitle = false; titleDraft = detail?.conversation.title || ''"
            />
            <v-btn
              size="small"
              color="primary"
              variant="text"
              class="title-save-btn"
              :icon="mobile ? 'mdi-check' : undefined"
              :aria-label="t('common.save')"
              @click="saveTitle"
            ><span v-if="!mobile">{{ t("common.save") }}</span></v-btn>
          </div>
          <button
            v-else
            type="button"
            class="chat-title-main chat-title-btn"
            :disabled="!selectedId"
            :title="t('chat.rename')"
            @click="editingTitle = true"
          >{{ detail?.conversation.title || t("chat.select") }}</button>
          <div v-if="!editingTitle" class="chat-title-sub text-truncate">{{ multiModel ? t("chat.multiModel") : (selectedPresetNames || t("chat.singleModel")) }}</div>
        </div>

        <v-menu v-if="branches.length > 1 && !editingTitle" location="bottom end" :offset="6">
          <template #activator="{ props: branchProps }">
            <button
              type="button"
              class="branch-chip"
              v-bind="branchProps"
              :title="t('chat.switchBranch')"
            >
              <v-icon icon="mdi-source-branch" size="16" />
              <span>{{ t("chat.branchN", { n: Math.max(1, branches.findIndex((b) => b.id === detail?.conversation.active_branch_id) + 1) }) }}</span>
              <span class="branch-chip-count">{{ branches.length }}</span>
              <v-icon icon="mdi-chevron-down" size="14" />
            </button>
          </template>
          <div class="branch-menu">
            <div class="branch-menu-label">{{ t("chat.switchBranch") }}</div>
            <button
              v-for="(b, i) in branches"
              :key="b.id"
              type="button"
              class="branch-menu-item"
              :class="{ active: b.id === detail?.conversation.active_branch_id }"
              @click="onSwitchBranch(b.id)"
            >
              <span class="branch-menu-item-text">
                {{ t("chat.branchN", { n: i + 1 }) }}
                <span v-if="b.fork_candidate_id" class="branch-menu-fork">{{ t("chat.branchFork") }}</span>
              </span>
              <v-icon
                v-if="b.id === detail?.conversation.active_branch_id"
                icon="mdi-check"
                size="18"
                class="branch-menu-check"
              />
            </button>
          </div>
        </v-menu>

        <v-btn
          icon="mdi-tune-variant"
          variant="text"
          density="compact"
          size="small"
          class="chat-settings-btn"
          :disabled="!selectedId"
          :title="t('chat.conversationSettings')"
          @click="settingsOpen = true"
        />
      </header>

      <div v-if="showWelcome" class="chat-empty stage">
        <div class="chat-empty-card">
          <div class="chat-empty-icon"><v-icon icon="mdi-auto-fix" size="28" /></div>
          <div class="chat-empty-title">{{ t("app.name") }}</div>
          <div class="chat-empty-hint">{{ presets.length ? t("chat.emptyHint") : t("chat.noModelsHint") }}</div>
          <div class="chat-empty-actions">
            <v-btn
              v-if="presets.length"
              color="primary"
              prepend-icon="mdi-plus"
              @click="createConversation"
            >{{ t("chat.new") }}</v-btn>
            <v-btn
              v-else
              color="primary"
              variant="tonal"
              prepend-icon="mdi-cog-outline"
              @click="router.push('/settings/providers')"
            >{{ t("chat.configureProviders") }}</v-btn>
          </div>
        </div>
      </div>

      <div v-if="bannerError" class="chat-error-bar">
        <v-alert type="error" variant="tonal" density="compact" closable @click:close="stream.clearError(); error = null">
          {{ bannerError }}
        </v-alert>
      </div>

      <div v-else-if="showThreadEmpty" class="chat-empty inline">
        <div class="chat-empty-card">
          <div class="chat-empty-icon"><v-icon icon="mdi-message-text-outline" size="24" /></div>
          <div class="chat-empty-title">{{ t("chat.emptyThread") }}</div>
        </div>
      </div>

      <div v-else ref="parentRef" class="chat-scroll" @scroll="onScroll">
        <div class="chat-scroll-inner">
          <div class="virtual-spacer" :style="{ height: `${totalSize}px` }">
            <div
              v-for="vItem in virtualItems"
              :key="String(vItem.key)"
              :ref="measureRef"
              :data-index="vItem.index"
              class="virtual-row"
              :style="{ transform: `translateY(${vItem.start}px)` }"
            >
              <template v-if="rowAt(vItem.index)">
                <div v-if="rowAt(vItem.index)!.kind === 'user'" class="msg-row msg-user">
                  <div class="msg-col">
                    <div class="msg-meta"><span>{{ t("chat.you") }}</span></div>
                    <div class="msg-bubble user-bubble">
                      <MarkdownView :text="(rowAt(vItem.index) as any).text" :cache-key="(rowAt(vItem.index) as any).cacheKey" />
                    </div>
                  </div>
                </div>

                <div v-else-if="rowAt(vItem.index)!.kind === 'pending_user'" class="msg-row msg-user">
                  <div class="msg-col">
                    <div class="msg-meta"><span>{{ t("chat.you") }}</span></div>
                    <div class="msg-bubble user-bubble pending-bubble">
                      <MarkdownView :text="(rowAt(vItem.index) as any).text" streaming />
                      <div v-if="(rowAt(vItem.index) as any).imageUrls?.length" class="d-flex flex-wrap ga-2 mt-2">
                        <img v-for="url in (rowAt(vItem.index) as any).imageUrls" :key="url" :src="url" class="attach-thumb" alt="" />
                      </div>
                    </div>
                  </div>
                </div>

                <div v-else-if="rowAt(vItem.index)!.kind === 'assistant'" class="msg-row msg-assistant">
                  <div class="msg-col">
                    <div class="msg-meta"><span>{{ t("app.name") }}</span></div>
                    <div class="msg-bubble assistant-bubble">
                      <ThinkingBlock
                        :text="(rowAt(vItem.index) as any).thinking"
                        :tokens="(rowAt(vItem.index) as any).thinkingTokens"
                        :duration-ms="(rowAt(vItem.index) as any).thinkingDurationMs"
                      />
                      <MarkdownView :text="(rowAt(vItem.index) as any).text" :cache-key="(rowAt(vItem.index) as any).cacheKey" />
                      <div v-if="hasUsageInfo(rowAt(vItem.index))" class="usage-line">
                        <span v-for="(part, pIdx) in formatUsageMeta(rowAt(vItem.index) as any)" :key="pIdx" class="usage-item">
                          <span class="usage-label">{{ part.label }}</span>
                          <span class="usage-value">{{ part.value }}</span>
                        </span>
                      </div>
                    </div>
                  </div>
                </div>

                <div v-else-if="rowAt(vItem.index)!.kind === 'round'" class="msg-row msg-assistant">
                  <div class="msg-col wide">
                    <div class="msg-meta"><span>{{ t("chat.multiModel") }}</span></div>
                    <div class="round-grid" :class="gridClass((rowAt(vItem.index) as any).candidates.length)">
                      <div
                        v-for="cand in (rowAt(vItem.index) as any).candidates"
                        :key="cand.messageId"
                        class="candidate-card"
                        :class="{ selected: cand.selected, dimmed: (rowAt(vItem.index) as any).selectedCandidateId && !cand.selected }"
                      >
                        <div class="msg-meta">
                          <span class="d-inline-flex align-center ga-2">
                            <span class="cand-badge">{{ cand.letter }}</span>
                            <span>{{ cand.label }}</span>
                            <span v-if="cand.selected" class="best-pill">{{ t("chat.best") }}</span>
                          </span>
                        </div>
                        <ThinkingBlock
                          :text="cand.thinking"
                          :tokens="cand.thinkingTokens"
                          :duration-ms="cand.thinkingDurationMs"
                        />
                        <MarkdownView :text="cand.text" :cache-key="cand.cacheKey" />
                        <div v-if="hasUsageInfo(cand)" class="usage-line">
                          <span v-for="(part, pIdx) in formatUsageMeta(cand)" :key="pIdx" class="usage-item">
                            <span class="usage-label">{{ part.label }}</span>
                            <span class="usage-value">{{ part.value }}</span>
                          </span>
                        </div>
                        <div class="d-flex flex-wrap ga-1 mt-2">
                          <v-btn v-if="cand.candidateId && !(rowAt(vItem.index) as any).selectedCandidateId" size="small" color="primary" variant="tonal" @click="actCandidate('commit', cand.candidateId)">{{ t("chat.pickBest") }}</v-btn>
                          <v-btn v-if="cand.candidateId" size="small" variant="text" @click="actCandidate('fork', cand.candidateId)">{{ t("common.fork") }}</v-btn>
                          <v-btn v-if="cand.candidateId && !(rowAt(vItem.index) as any).selectedCandidateId" size="small" variant="text" @click="actCandidate('retry', cand.candidateId)">{{ t("common.retry") }}</v-btn>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div v-else-if="rowAt(vItem.index)!.kind === 'streaming'" class="msg-row msg-assistant">
                  <div class="msg-col" :class="{ wide: stream.candidates.length > 1 }">
                    <div class="msg-meta"><span>{{ t("chat.streaming") }}</span></div>
                    <div v-if="stream.candidates.length <= 1" class="msg-bubble assistant-bubble">
                      <ThinkingBlock
                        :text="stream.candidates[0]?.thinking || ''"
                        :tokens="stream.candidates[0]?.reasoningTokens"
                        :duration-ms="stream.candidates[0]?.reasoningDurationMs"
                        :streaming="Boolean(stream.candidates[0]?.thinking) && !stream.candidates[0]?.text && !stream.candidates[0]?.done"
                      />
                      <MarkdownView :text="stream.candidates[0]?.text || ''" streaming />
                      <div v-if="!stream.candidates[0]?.text" class="text-medium-emphasis text-body-2">{{ t("chat.stopping") }}</div>
                    </div>
                    <div v-else class="round-grid" :class="gridClass(stream.candidates.length)">
                      <div v-for="cand in stream.candidates" :key="cand.candidateId" class="candidate-card">
                        <div class="msg-meta"><span class="cand-badge">{{ cand.slotLabel }}</span></div>
                        <ThinkingBlock
                          :text="cand.thinking"
                          :tokens="cand.reasoningTokens"
                          :duration-ms="cand.reasoningDurationMs"
                          :streaming="Boolean(cand.thinking) && !cand.text && !cand.done"
                        />
                        <MarkdownView :text="cand.text" streaming />
                        <div v-if="cand.error" class="text-error text-caption">{{ cand.error }}</div>
                      </div>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>
      <!-- Composer Bar -->
      <footer class="chat-composer">
        <div
          class="chat-composer-inner"
          :class="{ 'is-dragover': dragOver }"
          @dragover="onComposerDragOver"
          @dragleave="onComposerDragLeave"
          @drop="onComposerDrop"
        >
          <div v-if="attachments.length" class="composer-attach-row">
            <button
              v-for="(src, index) in attachments"
              :key="src"
              type="button"
              class="attach-thumb-btn"
              :title="t('common.delete')"
              @click="attachments.splice(index, 1)"
            >
              <img :src="src" class="attach-thumb" alt="" />
            </button>
          </div>

          <textarea
            v-model="draft"
            class="composer-textarea"
            rows="1"
            :placeholder="multiModel ? t('chat.multiPlaceholder') : t('chat.placeholder')"
            :disabled="!selectedId || stream.isStreaming"
            @keydown.enter.exact.prevent="send"
          />

          <div class="composer-bar">
            <div class="composer-bar-left">
              <v-menu v-model="addMenu" location="top start" :close-on-content-click="false">
                <template #activator="{ props: menuProps }">
                  <button type="button" class="composer-add-btn" v-bind="menuProps" :title="t('chat.add')">
                    <v-icon icon="mdi-plus" size="17" />
                  </button>
                </template>
                <v-card class="composer-add-menu" min-width="260" rounded="lg">
                  <div class="composer-add-section">{{ t("chat.addSection") }}</div>
                  <v-list density="compact">
                    <v-list-item prepend-icon="mdi-paperclip" :title="t('chat.addFiles')" :subtitle="t('chat.addFilesHint')" @click="pickAttachments" />
                    <v-list-item prepend-icon="mdi-tune-variant" :title="t('chat.conversationSettings')" :subtitle="t('chat.addSettingsHint')" @click="openConversationSettings" />
                  </v-list>
                  <div class="composer-add-section">{{ t("chat.pluginsSection") }}</div>
                  <v-list density="compact">
                    <v-list-item prepend-icon="mdi-connection" title="MCP" :subtitle="t('chat.addMcpHint')" @click="openMcpSettings" />
                    <v-list-item prepend-icon="mdi-server" :title="t('nav.providers')" :subtitle="t('chat.addProvidersHint')" @click="openProviderSettings" />
                  </v-list>
                </v-card>
              </v-menu>

              <v-menu v-model="modelMenu" location="top start" :close-on-content-click="false">
                <template #activator="{ props: menuProps }">
                  <button type="button" class="composer-model-chip" v-bind="menuProps" :disabled="!presets.length">
                    <v-icon icon="mdi-cube-outline" size="15" />
                    <span class="composer-model-label">{{ modelButtonLabel }}</span>
                    <v-icon icon="mdi-chevron-up" size="14" />
                  </button>
                </template>
                <div class="model-menu-compact">
                  <div class="model-menu-head">
                    <span class="model-menu-title">{{ t("chat.selectModel") }}</span>
                    <span class="model-menu-count">{{ selectedModels.length }}</span>
                  </div>
                  <div class="model-menu-items">
                    <button
                      v-for="preset in presets"
                      :key="preset.id"
                      type="button"
                      class="model-menu-item"
                      :class="{ selected: selectedModels.includes(preset.id) }"
                      @click="toggleModel(preset.id)"
                    >
                      <span class="model-menu-check" :class="{ on: selectedModels.includes(preset.id) }">
                        <v-icon v-if="selectedModels.includes(preset.id)" icon="mdi-check" size="12" />
                      </span>
                      <span class="model-menu-name">{{ preset.name }}</span>
                    </button>
                  </div>
                </div>
              </v-menu>

              <v-menu v-model="modeMenu" location="top start">
                <template #activator="{ props: menuProps }">
                  <button type="button" class="composer-mode-chip composer-mode-button" :title="t('chat.runtimeMode')" v-bind="menuProps">
                    <v-icon icon="mdi-message-text-outline" size="15" />
                    <span class="composer-mode-text">{{ t("chat.conversationMode") }}</span>
                    <v-icon icon="mdi-chevron-up" size="14" class="composer-mode-chevron" />
                  </button>
                </template>
                <div class="runtime-mode-menu">
                  <div class="runtime-mode-title">{{ t("chat.runtimeMode") }}</div>
                  <button type="button" class="runtime-mode-item active" @click="modeMenu = false">
                    <span class="runtime-mode-icon"><v-icon icon="mdi-message-text-outline" size="17" /></span>
                    <span class="runtime-mode-copy">
                      <strong>{{ t("chat.conversationMode") }}</strong>
                      <small>{{ t("chat.conversationModeHint") }}</small>
                    </span>
                    <v-icon icon="mdi-check" size="16" />
                  </button>
                  <button type="button" class="runtime-mode-item reserved" @click="openAgentEntry">
                    <span class="runtime-mode-icon"><v-icon icon="mdi-robot-outline" size="17" /></span>
                    <span class="runtime-mode-copy">
                      <strong>{{ t("chat.agentMode") }}</strong>
                      <small>{{ t("chat.agentModeHint") }}</small>
                    </span>
                    <span class="runtime-mode-badge">{{ t("chat.comingSoon") }}</span>
                  </button>
                </div>
              </v-menu>
            </div>

            <div class="composer-bar-right">
              <button
                v-if="stream.isStreaming"
                type="button"
                class="composer-send stop"
                :title="t('chat.stop')"
                @click="stopGeneration"
              >
                <span class="composer-stop-square" />
              </button>
              <button
                v-else
                type="button"
                class="composer-send"
                :disabled="!canSend"
                :title="t('chat.send')"
                @click="send"
              >
                <v-icon icon="mdi-arrow-up" size="18" />
              </button>
            </div>
          </div>

          <div v-if="dragOver" class="composer-drop-hint">{{ t("chat.dropImages") }}</div>
        </div>
      </footer>
    </section>
    <!-- Conversation Settings Modal -->
    <v-dialog v-model="settingsOpen" max-width="500">
      <v-card rounded="xl">
        <v-card-title class="pt-4 px-5 font-weight-bold">{{ t("chat.conversationSettings") }}</v-card-title>
        <v-card-text class="px-5">
          <div class="mb-4">
            <div class="d-flex align-center justify-space-between ga-3 mb-1">
              <label class="compact-field-label">{{ t("chat.assistantRole") }}</label>
              <v-btn size="x-small" variant="text" prepend-icon="mdi-account-cog-outline" @click="router.push('/settings/assistants'); settingsOpen = false">
                {{ t("chat.manageAssistants") }}
              </v-btn>
            </div>
            <v-select
              :model-value="selectedRoleId"
              :items="roleStore.roles"
              item-title="name"
              item-value="id"
              density="compact"
              variant="outlined"
              hide-details
              @update:model-value="applyConversationRole"
            />
            <div class="settings-field-hint mt-1">{{ t("chat.assistantRoleHint") }}</div>
          </div>
          <div class="mb-4">
            <label class="compact-field-label mb-1">{{ t("chat.temperature") }}: {{ temperature }}</label>
            <v-slider v-model="temperature" :min="0" :max="2" :step="0.1" thumb-label color="primary" hide-details />
          </div>
          <div>
            <label class="compact-field-label mb-1">{{ t("chat.systemPrompt") }}</label>
            <textarea v-model="systemPrompt" class="settings-input" rows="3" style="height:auto;padding:8px 12px;" />
          </div>
        </v-card-text>
        <v-card-actions class="px-5 pb-4">
          <v-spacer />
          <v-btn variant="text" @click="settingsOpen = false">{{ t("common.cancel") }}</v-btn>
          <v-btn color="primary" @click="saveSettings">{{ t("common.save") }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>
