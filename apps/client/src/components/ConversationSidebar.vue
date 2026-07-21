<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import type { Conversation, SessionInfo } from "../api";
import { api, errMessage } from "../api";
import { useRoleStore } from "../stores/roles";
import { usePreferencesStore, type ThemeMode } from "../stores/preferences";

const props = defineProps<{
  conversations: Conversation[];
  selectedId: string | null;
  filter: string;
  mobile?: boolean;
  newLabel: string;
  searchLabel: string;
  emptyLabel: string;
  multiLabel: string;
  singleLabel: string;
  todayLabel: string;
  yesterdayLabel: string;
  earlierLabel: string;
}>();

const emit = defineEmits<{
  "update:filter": [value: string];
  select: [id: string];
  create: [];
  createWithRole: [roleId: string];
  export: [id: string];
  remove: [id: string];
}>();

const { t } = useI18n();
const route = useRoute();
void route.path;
const router = useRouter();
const roleStore = useRoleStore();
const preferences = usePreferencesStore();

const themeItems = computed(() => [
  { title: t("common.system"), value: "system" as ThemeMode, icon: "mdi-theme-light-dark" },
  { title: t("common.light"), value: "light" as ThemeMode, icon: "mdi-white-balance-sunny" },
  { title: t("common.dark"), value: "dark" as ThemeMode, icon: "mdi-weather-night" },
]);



const activeRoleId = ref<string>(roleStore.defaultRoleId);
const sidebarWidth = ref(parseInt(localStorage.getItem("aerina.unifiedSidebarWidth") || "220"));

// Profile Session State
const session = ref<SessionInfo | null>(null);
const avatarUrl = ref<string | null>(null);
const menuOpen = ref(false);
const accountError = ref<string | null>(null);

const profile = computed(() => session.value?.profile ?? null);
const initial = computed(() => {
  const name = profile.value?.display_name?.trim() || "?";
  return name.slice(0, 1).toUpperCase();
});

function profileInitial(name: string) {
  const n = name.trim() || "?";
  return n.slice(0, 1).toUpperCase();
}

async function refreshAvatar() {
  const id = profile.value?.id;
  if (!id || !profile.value?.avatar_path) {
    avatarUrl.value = null;
    return;
  }
  avatarUrl.value = await api.getProfileAvatarDataUrl(id);
}

async function loadSession() {
  try {
    session.value = await api.sessionInfo();
    await refreshAvatar();
  } catch (e) {
    console.error(e);
  }
}

async function onSwitchProfile(id: string) {
  if (id === profile.value?.id) {
    menuOpen.value = false;
    return;
  }
  accountError.value = null;
  try {
    session.value = await api.switchProfile(id);
    menuOpen.value = false;
    await refreshAvatar();
    window.dispatchEvent(new CustomEvent("aerina:session-changed"));
  } catch (e) {
    accountError.value = errMessage(e);
  }
}

// Resizing logic
let isResizing = false;
function startResize(_e: MouseEvent) {
  isResizing = true;
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", stopResize);
}

function onMouseMove(e: MouseEvent) {
  if (!isResizing) return;
  const newWidth = Math.max(180, Math.min(320, e.clientX));
  sidebarWidth.value = newWidth;
  localStorage.setItem("aerina.unifiedSidebarWidth", String(newWidth));
}

function stopResize() {
  isResizing = false;
  document.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseup", stopResize);
}

onMounted(() => {
  void loadSession();
});

onUnmounted(() => {
  stopResize();
});

// All assistants with Default Assistant pinned at index 0
const allAssistants = computed(() => {
  const defId = roleStore.defaultRoleId;
  const defRole = roleStore.defaultRole;
  const others = roleStore.roles.filter((r) => r.id !== defId);
  return [defRole, ...others];
});

// Currently active Assistant
const activeRole = computed(() => {
  return roleStore.roles.find((r) => r.id === activeRoleId.value) || roleStore.defaultRole;
});

function countForRole(roleId: string) {
  const defId = roleStore.defaultRoleId;
  return props.conversations.filter((c) => {
    const rId = roleStore.convRoles[c.id] || "default";
    if (roleId === defId || roleId === "default") {
      return rId === defId || rId === "default";
    }
    return rId === roleId;
  }).length;
}

const activeRoleConversations = computed(() => {
  const targetId = activeRoleId.value;
  const defId = roleStore.defaultRoleId;

  return props.conversations.filter((c) => {
    const rId = roleStore.convRoles[c.id] || "default";
    if (targetId === defId || targetId === "default") {
      return rId === defId || rId === "default";
    }
    return rId === targetId;
  });
});

function dayStart(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

const activeGroups = computed(() => {
  const now = new Date();
  const today = dayStart(now);
  const yesterday = today - 86400000;
  const buckets: Array<{ key: string; label: string; items: Conversation[] }> = [
    { key: "today", label: props.todayLabel, items: [] },
    { key: "yesterday", label: props.yesterdayLabel, items: [] },
    { key: "earlier", label: props.earlierLabel, items: [] },
  ];
  for (const item of activeRoleConversations.value) {
    const t = dayStart(new Date(item.updated_at || item.created_at));
    if (t >= today) buckets[0].items.push(item);
    else if (t >= yesterday) buckets[1].items.push(item);
    else buckets[2].items.push(item);
  }
  return buckets.filter((b) => b.items.length);
});

watch(
  () => props.selectedId,
  (newId) => {
    if (newId) {
      const rId = roleStore.convRoles[newId] || roleStore.defaultRoleId;
      if (rId) activeRoleId.value = rId;
    }
  },
  { immediate: true },
);

function createNew() {
  if (activeRoleId.value === roleStore.defaultRoleId || activeRoleId.value === "default") {
    emit("create");
  } else {
    emit("createWithRole", activeRoleId.value);
  }
}
</script>

<template>
  <aside
    class="unified-sidebar"
    :class="{ mobile }"
    :style="mobile ? {} : { width: sidebarWidth + 'px', flex: `0 0 ${sidebarWidth}px` }"
  >
    <!-- Header: Logo Brand & Titlebar Drag Region -->
    <div v-if="!mobile" class="unified-header" data-tauri-drag-region>
      <div class="unified-brand">
        <img class="unified-logo" src="/brand/logo-mark.png" alt="Aerina" />
        <span class="unified-title font-weight-bold">Aerina</span>
        <span class="unified-version text-caption text-medium-emphasis ms-1">v0.1.0</span>
      </div>
    </div>

    <!-- Quick Action: + New Chat Button & Search -->
    <div class="unified-actions-block px-2 py-1">
      <v-btn
        v-if="!mobile"
        block
        color="primary"
        variant="flat"
        class="unified-new-btn mb-1"
        prepend-icon="mdi-plus"
        @click="createNew"
      >
        {{ newLabel }}
      </v-btn>

      <div class="chat-search-row">
        <label class="chat-search-box">
          <v-icon icon="mdi-magnify" size="14" class="chat-search-icon" />
          <input
            class="chat-search-input"
            type="search"
            :value="filter"
            :placeholder="searchLabel"
            autocomplete="off"
            spellcheck="false"
            @input="emit('update:filter', ($event.target as HTMLInputElement).value)"
          />
        </label>

      </div>
    </div>

    <!-- Scrollable Body: Assistant Roles Chips + Conversation List -->
    <div class="unified-scroll-body">
      <!-- Assistant Roles Horizontal Chips -->
      <div class="unified-section-label px-2 pt-1">
        <span>助手角色</span>
      </div>

      <div class="assistant-chips-scroller px-2 py-1">
        <button
          v-for="role in allAssistants"
          :key="role.id"
          type="button"
          class="assistant-chip-item"
          :class="{ active: activeRoleId === role.id }"
          @click="activeRoleId = role.id"
        >
          <v-icon :icon="role.icon" size="13" />
          <span class="chip-name">{{ role.name }}</span>
          <span v-if="role.id === roleStore.defaultRoleId" class="chip-def-dot" title="默认助手" />
          <span class="chip-count">{{ countForRole(role.id) }}</span>
        </button>
      </div>

      <!-- Active Assistant's Conversations List -->
      <div class="unified-section-label px-2 pt-2">
        <span>{{ activeRole.name }} 的对话</span>
      </div>

      <div class="unified-conv-list px-1 py-0">
        <template v-for="group in activeGroups" :key="group.key">
          <div class="conv-group-label px-2 pt-1 mb-0">{{ group.label }}</div>
          <button
            v-for="item in group.items"
            :key="item.id"
            type="button"
            class="conv-item"
            :class="{ active: selectedId === item.id }"
            @click="emit('select', item.id)"
          >
            <div class="conv-item-main">
              <div class="conv-item-title">{{ item.title }}</div>
              <div class="conv-item-sub">
                <span class="conv-mode-dot" :class="item.mode === 'sbs' ? 'multi' : 'single'" />
                {{ item.mode === "sbs" ? multiLabel : singleLabel }}
              </div>
            </div>
            <div class="conv-item-actions" @click.stop>
              <v-btn icon="mdi-download-outline" size="x-small" variant="text" @click="emit('export', item.id)" />
              <v-btn icon="mdi-delete-outline" size="x-small" variant="text" @click="emit('remove', item.id)" />
            </div>
          </button>
        </template>

        <div v-if="!activeRoleConversations.length" class="empty-conv-placeholder py-4">
          <v-icon :icon="activeRole.icon" size="24" class="mb-1 text-medium-emphasis" />
          <div class="text-body-2 font-weight-medium">暂无对话</div>
          <div class="text-caption text-medium-emphasis mt-1">点击上方 + 新建对话</div>
        </div>
      </div>
    </div>

    <!-- Footer: User Profile Activator -->
    <div class="unified-footer px-1 py-1 border-t">
      <v-menu
        v-model="menuOpen"
        location="top start"
        :close-on-content-click="false"
        :offset="8"
        content-class="profile-menu-overlay"
      >
        <template #activator="{ props: menuProps }">
          <button
            type="button"
            class="unified-profile-btn"
            v-bind="menuProps"
          >
            <span class="unified-avatar-box">
              <img v-if="avatarUrl" :src="avatarUrl" alt="" />
              <span v-else>{{ initial }}</span>
            </span>
            <span class="unified-profile-name text-truncate">{{ profile?.display_name || t("profile.title") }}</span>
            <v-icon icon="mdi-unfold-more-horizontal" size="13" class="ms-auto text-medium-emphasis" />
          </button>
        </template>

        <div class="profile-menu">
          <div class="profile-menu-current">
            <span class="profile-menu-current-avatar">
              <img v-if="avatarUrl" :src="avatarUrl" alt="" />
              <template v-else>{{ initial }}</template>
            </span>
            <span class="profile-menu-current-meta">
              <span class="profile-menu-current-name">{{ profile?.display_name || t("profile.title") }}</span>
              <span class="profile-menu-current-subtitle">{{ t("profile.localHintShort") }}</span>
            </span>
          </div>

          <div v-if="accountError" class="profile-menu-error">{{ accountError }}</div>

          <div class="profile-menu-section">
            <div class="profile-menu-label">{{ t("profile.switch") }}</div>
            <button
              v-for="p in session?.profiles || []"
              :key="p.id"
              type="button"
              class="profile-menu-item"
              :class="{ active: p.id === profile?.id }"
              @click="onSwitchProfile(p.id)"
            >
              <span class="profile-menu-item-avatar">{{ profileInitial(p.display_name) }}</span>
              <span class="profile-menu-item-text">{{ p.display_name }}</span>
              <v-icon v-if="p.id === profile?.id" icon="mdi-check" size="14" class="profile-menu-check" />
            </button>
          </div>

          <div class="profile-menu-divider" />

          <div class="profile-menu-section">
            <div class="profile-menu-label">{{ t("appearance.theme") }}</div>
            <div class="profile-theme-row" role="radiogroup" :aria-label="t('appearance.theme')">
              <button
                v-for="item in themeItems"
                :key="item.value"
                type="button"
                class="profile-theme-btn"
                role="radio"
                :aria-checked="preferences.themeMode === item.value"
                :class="{ active: preferences.themeMode === item.value }"
                :title="item.title"
                @click="preferences.setThemeMode(item.value)"
              >
                <v-icon :icon="item.icon" size="15" />
                <span>{{ item.title }}</span>
              </button>
            </div>
          </div>

          <div class="profile-menu-divider" />

          <div class="profile-menu-section">
            <button type="button" class="profile-menu-item action" @click="menuOpen = false; router.push('/profile')">
              <v-icon icon="mdi-account-circle-outline" size="15" class="profile-menu-leading" />
              <span class="profile-menu-item-text">{{ t("nav.profile") }}</span>
            </button>
            <button type="button" class="profile-menu-item action" @click="menuOpen = false; router.push('/settings/appearance')">
              <v-icon icon="mdi-palette-outline" size="15" class="profile-menu-leading" />
              <span class="profile-menu-item-text">{{ t("nav.appearance") }}</span>
            </button>
            <button type="button" class="profile-menu-item action" @click="menuOpen = false; router.push('/settings')">
              <v-icon icon="mdi-cog-outline" size="15" class="profile-menu-leading" />
              <span class="profile-menu-item-text">{{ t("nav.settings") }}</span>
            </button>
            <button type="button" class="profile-menu-item action" @click="menuOpen = false; router.push('/ranking')">
              <v-icon icon="mdi-trophy-outline" size="15" class="profile-menu-leading" />
              <span class="profile-menu-item-text">{{ t("nav.ranking") }}</span>
            </button>
          </div>
        </div>
      </v-menu>
    </div>

    <!-- Mouse Drag Resizer for Sidebar Width -->
    <div
      v-if="!mobile"
      class="sidebar-col-resizer"
      title="拖拽调整侧边栏宽度"
      @mousedown.prevent="startResize"
    />
  </aside>
</template>
