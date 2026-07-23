import { defineComponent, h } from "vue";
import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import AppShell from "../layouts/AppShell.vue";

// Lightweight placeholders — real Chat/Profile/Ranking stay mounted in AppShell.
const Blank = defineComponent({ name: "BlankRoute", setup: () => () => h("div", { class: "d-none" }) });

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: AppShell,
      children: [
        {
          path: "",
          name: "chat",
          component: Blank,
          meta: { page: "chat" },
        },
        { path: "arena", redirect: "/ranking" },
        { path: "stats", redirect: "/profile" },
        {
          path: "profile",
          name: "profile",
          component: Blank,
          meta: { page: "profile" },
        },
        {
          path: "ranking",
          name: "ranking",
          component: Blank,
          meta: { page: "ranking" },
        },
        {
          path: "settings",
          name: "settings",
          component: () => import("../pages/settings/SettingsHubPage.vue"),
          meta: { keepAlive: true },
        },
        {
          path: "settings/providers",
          name: "providers",
          component: () => import("../pages/settings/ProvidersPage.vue"),
        },
        {
          path: "settings/providers/:providerId",
          name: "provider-detail",
          component: () => import("../pages/settings/ProviderDetailPage.vue"),
        },
        {
          path: "settings/mcp",
          name: "mcp",
          component: () => import("../pages/settings/McpPage.vue"),
        },
        {
          path: "settings/profile",
          name: "profile-settings",
          component: () => import("../pages/settings/ProfileSettingsPage.vue"),
        },
        {
          path: "settings/assistants",
          name: "assistants",
          component: () => import("../pages/settings/AssistantsPage.vue"),
        },
        {
          path: "settings/appearance",
          name: "appearance",
          component: () => import("../pages/settings/AppearancePage.vue"),
        },
        { path: "compare", redirect: "/" },
        { path: "models", redirect: "/settings/providers" },
      ] satisfies RouteRecordRaw[],
    },
    {
      path: "/html-preview/:previewId",
      name: "html-preview",
      component: () => import("../pages/HtmlPreviewPage.vue"),
      meta: { title: "HTML Preview" },
    },
    {
      path: "/login",
      name: "login",
      component: () => import("../pages/LoginPage.vue"),
      meta: { title: "登录" },
    },
  ],
});
