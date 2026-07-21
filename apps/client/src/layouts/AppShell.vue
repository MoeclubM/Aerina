<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import ChatPage from "../pages/ChatPage.vue";

const ProfilePage = defineAsyncComponent(() => import("../pages/ProfilePage.vue"));
const RankingPage = defineAsyncComponent(() => import("../pages/RankingPage.vue"));

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

const windowWidth = ref(window.innerWidth);
const handleResize = () => { windowWidth.value = window.innerWidth; };
const mobile = computed(() => windowWidth.value < 960);

const items = computed(() => [
  { to: "/", title: t("nav.chat"), icon: "mdi-message-text-outline", exact: true },
  { to: "/ranking", title: t("nav.ranking"), icon: "mdi-trophy-outline" },
  { to: "/profile", title: t("nav.profile"), icon: "mdi-account-circle-outline" },
]);

const seen = reactive({ chat: true, ranking: false, profile: false, settings: false });

const page = computed<"chat" | "ranking" | "profile" | "settings">(() => {
  if (route.path.startsWith("/settings") || route.name === "settings") return "settings";
  if (route.name === "ranking" || route.path.startsWith("/ranking")) return "ranking";
  if (route.name === "profile" || route.path.startsWith("/profile")) return "profile";
  return "chat";
});

watch(page, (p) => {
  if (p === "chat") seen.chat = true;
  if (p === "ranking") seen.ranking = true;
  if (p === "profile") seen.profile = true;
  if (p === "settings") seen.settings = true;
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
  if (route.name === "appearance") return t("nav.appearance");
  if (route.name === "providers" || route.name === "provider-detail") return t("nav.providers");
  if (route.name === "mcp") return t("nav.mcp");
  if (page.value === "ranking") return t("nav.ranking");
  if (page.value === "profile") return t("nav.profile");
  if (page.value === "settings") return t("nav.settings");
  return t("app.name");
});

function triggerMobileMenu() {
  window.dispatchEvent(new CustomEvent("aerina:toggle-mobile-drawer"));
}

function triggerMobileNewChat() {
  window.dispatchEvent(new CustomEvent("aerina:new-chat"));
}

const mobileSubpageActive = ref(false);

function handleSubpageChange(e: Event) {
  const customEv = e as CustomEvent<{ active: boolean }>;
  mobileSubpageActive.value = customEv.detail?.active ?? false;
}

onMounted(() => {
  window.addEventListener("aerina:mobile-subpage-changed", handleSubpageChange);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("aerina:mobile-subpage-changed", handleSubpageChange);
});
</script>

<template>
    <!-- Mobile top bar: solid frosted glass, status bar safe area, menu & new chat buttons -->
  <v-app-bar v-if="mobile && !mobileSubpageActive" flat density="compact" class="mobile-app-bar">
    <template #prepend>
      <v-btn
        icon="mdi-menu"
        variant="text"
        density="comfortable"
        class="ms-1"
        title="打开菜单"
        @click="triggerMobileMenu"
      />
    </template>

    <v-app-bar-title class="text-body-1 font-weight-bold text-truncate px-1">
      {{ mobileTitle }}
    </v-app-bar-title>

    <template #append>
      <v-btn
        icon="mdi-plus"
        variant="tonal"
        color="primary"
        size="small"
        class="me-2"
        :title="t('chat.new')"
        @click="triggerMobileNewChat"
      />
    </template>
  </v-app-bar><v-main class="app-main" :class="{ 'has-tab-bar': mobile && !mobileSubpageActive }">
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
          <keep-alive :include="['SettingsHubPage','ProvidersPage','McpPage','AppearancePage','ProviderDetailPage']" :max="8">
            <component :is="Component" :key="r.meta.keepAlive ? String(r.name) : r.fullPath" />
          </keep-alive>
        </router-view>
      </div>
    </div>
  </v-main>

  <!-- Apple HIG bottom tab bar: 3 tabs -->
  <nav v-if="mobile && !mobileSubpageActive" class="aerina-tab-bar">
    <button
      v-for="item in items"
      :key="item.to"
      type="button"
      class="tab-bar-item"
      :class="{ active: item.exact ? route.path === item.to : route.path === item.to || route.path.startsWith(item.to + '/') }"
      @click="router.push(item.to)"
    >
      <v-icon :icon="item.icon" size="22" />
      <span class="tab-bar-label">{{ item.title }}</span>
    </button>
  </nav>
</template>