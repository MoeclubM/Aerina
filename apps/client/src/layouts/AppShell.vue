<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import ChatPage from "../pages/ChatPage.vue";
import ConversationSidebar from "../components/ConversationSidebar.vue";
import AgentEntryPanel from "../components/AgentEntryPanel.vue";

const ProfilePage = defineAsyncComponent(() => import("../pages/ProfilePage.vue"));
const RankingPage = defineAsyncComponent(() => import("../pages/RankingPage.vue"));

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

const windowWidth = ref(window.innerWidth);
const handleResize = () => { windowWidth.value = window.innerWidth; };
const mobile = computed(() => windowWidth.value < 680);
const mobileSubpageActive = ref(false);
const agentSheetOpen = ref(false);

const items = computed(() => [
  { key: "chat", to: "/", title: t("nav.chat"), icon: "mdi-message-text-outline", exact: true },
  { key: "agent", title: t("chat.agentTab"), icon: "mdi-robot-outline", reserved: true },
  { key: "settings", to: "/settings", title: t("nav.settings"), icon: "mdi-cog-outline" },
]);

const seen = reactive({ chat: true, ranking: false, profile: false, settings: false });

const page = computed<"chat" | "ranking" | "profile" | "settings">(() => {
  if (route.path.startsWith("/settings") || route.name === "settings") return "settings";
  if (route.name === "ranking" || route.path.startsWith("/ranking")) return "ranking";
  if (route.name === "profile" || route.path.startsWith("/profile")) return "profile";
  return "chat";
});

const mobileSettingsSubpage = computed(() => mobile.value && page.value === "settings" && route.name !== "settings");
const showMobileTabs = computed(() => mobile.value && !mobileSubpageActive.value && !mobileSettingsSubpage.value);

watch(page, (p) => {
  if (p === "chat") seen.chat = true;
  if (p === "ranking") seen.ranking = true;
  if (p === "profile") seen.profile = true;
  if (p === "settings") seen.settings = true;
  if (p !== "chat") mobileSubpageActive.value = false;
}, { immediate: true });

onMounted(() => {
  window.addEventListener("resize", handleResize);
  const warm = () => {
    void import("../pages/ProfilePage.vue");
    void import("../pages/RankingPage.vue");
    void import("../pages/settings/SettingsHubPage.vue");
  };
  if ("requestIdleCallback" in window) {
    (window as Window & { requestIdleCallback: (cb: () => void, opts?: { timeout: number }) => void }).requestIdleCallback(warm, { timeout: 1200 });
  } else {
    setTimeout(warm, 300);
  }
});

const mobileTitle = computed(() => {
  if (route.name === "profile-settings") return t("settings.profileTitle");
  if (route.name === "assistants") return t("settings.assistantRolesTitle");
  if (route.name === "appearance") return t("nav.appearance");
  if (route.name === "providers" || route.name === "provider-detail") return t("nav.providers");
  if (route.name === "mcp") return t("nav.mcp");
  if (page.value === "ranking") return t("nav.ranking");
  if (page.value === "profile") return t("nav.profile");
  if (page.value === "settings") return t("nav.settings");
  return t("app.name");
});


function mobileItemActive(item: { to?: string; exact?: boolean }) {
  if (!item.to) return false;
  return item.exact
    ? route.path === item.to
    : route.path === item.to || route.path.startsWith(`${item.to}/`);
}

function activateMobileItem(item: { to?: string; reserved?: boolean }) {
  if (item.reserved) {
    agentSheetOpen.value = true;
    return;
  }
  if (item.to) void router.push(item.to);
}

function handleAgentEntryOpen() {
  agentSheetOpen.value = true;
}

function handleSubpageChange(e: Event) {
  const customEv = e as CustomEvent<{ active: boolean }>;
  mobileSubpageActive.value = customEv.detail?.active ?? false;
}

onMounted(() => {
  window.addEventListener("aerina:mobile-subpage-changed", handleSubpageChange);
  window.addEventListener("aerina:open-agent-entry", handleAgentEntryOpen);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("aerina:mobile-subpage-changed", handleSubpageChange);
  window.removeEventListener("aerina:open-agent-entry", handleAgentEntryOpen);
});
</script>

<template>
    <!-- Mobile top bar: compact brand header with page-specific actions -->
  <v-app-bar v-if="mobile && page !== 'chat' && !mobileSettingsSubpage && !mobileSubpageActive" flat density="compact" class="mobile-app-bar">
    <template #prepend>
      <img class="mobile-app-logo ms-3" src="/brand/logo-mark.png" alt="" />
    </template>


    <v-app-bar-title class="text-body-1 font-weight-bold text-truncate px-1">
      {{ mobileTitle }}
    </v-app-bar-title>


  </v-app-bar>

  <div class="app-shell-frame">
    <ConversationSidebar
      v-if="!mobile && page !== 'chat'"
      navigation-only
      :conversations="[]"
      :selected-id="null"
      filter=""
      :new-label="t('chat.new')"
      :search-label="t('chat.search')"
      :multi-label="t('chat.multiModel')"
      :single-label="t('chat.singleModel')"
      :today-label="t('chat.today')"
      :yesterday-label="t('chat.yesterday')"
      :earlier-label="t('chat.earlier')"
    />

    <v-main class="app-main" :class="{ 'has-tab-bar': showMobileTabs }">
      <div class="page-host">
      <div v-show="page === 'chat'" class="page-layer" :class="{ active: page === 'chat' }" :inert="page !== 'chat'">
        <ChatPage v-if="seen.chat" :active="page === 'chat'" />
      </div>
      <div v-show="page === 'ranking'" class="page-layer" :class="{ active: page === 'ranking' }" :inert="page !== 'ranking'">
        <RankingPage v-if="seen.ranking" :active="page === 'ranking'" />
      </div>
      <div v-show="page === 'profile'" class="page-layer" :class="{ active: page === 'profile' }" :inert="page !== 'profile'">
        <ProfilePage v-if="seen.profile" :active="page === 'profile'" />
      </div>
      <div v-show="page === 'settings'" class="page-layer" :class="{ active: page === 'settings' }" :inert="page !== 'settings'">
        <router-view v-slot="{ Component, route: r }">
          <keep-alive :include="['SettingsHubPage','ProvidersPage','McpPage','ProfileSettingsPage','AssistantsPage','AppearancePage','ProviderDetailPage']" :max="8">
            <component :is="Component" :key="r.meta.keepAlive ? String(r.name) : r.fullPath" />
          </keep-alive>
        </router-view>
      </div>
      </div>
    </v-main>
  </div>

  <!-- Xiaomi-style mobile bottom navigation: 3 tabs -->
  <nav v-if="showMobileTabs" class="aerina-tab-bar">
    <button
      v-for="item in items"
      :key="item.key"
      type="button"
      class="tab-bar-item"
      :class="{ active: mobileItemActive(item), reserved: item.reserved }"
      @click="activateMobileItem(item)"
    >
      <v-icon :icon="item.icon" size="22" />
      <span class="tab-bar-label">{{ item.title }}</span>
    </button>
  </nav>

  <v-bottom-sheet v-if="mobile" v-model="agentSheetOpen" class="agent-entry-sheet-wrap">
    <AgentEntryPanel @close="agentSheetOpen = false" />
  </v-bottom-sheet>
  <v-dialog v-else v-model="agentSheetOpen" max-width="460">
    <AgentEntryPanel desktop @close="agentSheetOpen = false" />
  </v-dialog>
</template>

<style scoped>
.agent-entry-sheet-wrap :deep(.v-bottom-sheet__content) {
  width: min(100%, 520px);
  margin-inline: auto;
}
.app-shell-frame {
  flex: 1 1 auto;
  display: flex;
  width: 100%;
  min-width: 0;
  height: 100%;
  min-height: 0;
}

.app-main {
  flex: 1 1 auto;
  width: 100%;
  min-width: 0;
  min-height: 0;
}
</style>
