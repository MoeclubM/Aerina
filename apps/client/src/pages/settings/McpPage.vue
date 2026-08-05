<script setup lang="ts">
defineOptions({ name: "McpPage" });

import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, errMessage, type McpToolInfo, type McpTransport } from "../../api";

const { t } = useI18n();
const router = useRouter();
const servers = ref<Awaited<ReturnType<typeof api.listMcpServers>>>([]);
const tools = ref<McpToolInfo[]>([]);
const showForm = ref(false);
const error = ref<string | null>(null);
const testResult = ref<Record<string, string>>({});
const form = ref({
  name: "",
  transport: "streamable_http" as McpTransport,
  url: "",
  headerKey: "Authorization",
  headerValue: "",
  enabled: true,
});

async function refresh() {
  servers.value = await api.listMcpServers();
}

async function save() {
  error.value = null;
  try {
    await api.upsertMcpServer({
      name: form.value.name || "MCP Server",
      transport: form.value.transport,
      url: form.value.url,
      headers:
        form.value.headerKey && form.value.headerValue
          ? [[form.value.headerKey, form.value.headerValue]]
          : [],
      enabled: form.value.enabled,
    });
    showForm.value = false;
    form.value = {
      name: "",
      transport: "streamable_http",
      url: "",
      headerKey: "Authorization",
      headerValue: "",
      enabled: true,
    };
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  }
}

async function remove(id: string) {
  await api.deleteMcpServer(id);
  await refresh();
}

async function test(id: string) {
  try {
    testResult.value[id] = await api.testMcpServer(id);
  } catch (e) {
    testResult.value[id] = errMessage(e);
  }
}

async function listTools() {
  error.value = null;
  try {
    tools.value = await api.listMcpTools();
  } catch (e) {
    error.value = errMessage(e);
  }
}

onMounted(refresh);
</script>

<template>
  <v-container class="settings-page mcp-page">
    <div class="mcp-header d-flex align-center ga-2 mb-4 flex-wrap">
      <button
        type="button"
        class="settings-mobile-back"
        :aria-label="t('common.back')"
        @click="router.push('/settings')"
      >
        <v-icon icon="mdi-arrow-left" size="22" />
      </button>
      <div class="flex-grow-1" style="min-width: 0">
        <div class="mcp-title text-h5 mb-1">{{ t("mcp.title") }}</div>
        <div class="mcp-desc text-body-2 text-medium-emphasis">{{ t("mcp.desc") }}</div>
      </div>
      <v-btn color="primary" variant="tonal" prepend-icon="mdi-wrench" @click="listTools">{{ t("mcp.listTools") }}</v-btn>
      <v-btn color="primary" prepend-icon="mdi-plus" @click="showForm = !showForm">{{ t("mcp.add") }}</v-btn>
    </div>

    <v-alert v-if="error" type="error" variant="tonal" class="mb-4" :text="error" />

    <v-expand-transition>
      <v-card v-if="showForm" class="settings-panel mb-4">
        <v-card-text class="d-flex flex-column ga-3">
          <v-text-field v-model="form.name" :label="t('mcp.name')" />
          <v-select
            v-model="form.transport"
            :items="['streamable_http', 'sse']"
            :label="t('mcp.transport')"
          />
          <v-text-field v-model="form.url" :label="t('mcp.url')" />
          <v-row dense>
            <v-col cols="12" sm="6">
              <v-text-field v-model="form.headerKey" :label="t('mcp.headerKey')" />
            </v-col>
            <v-col cols="12" sm="6">
              <v-text-field v-model="form.headerValue" :label="t('mcp.headerValue')" />
            </v-col>
          </v-row>
          <v-switch v-model="form.enabled" :label="t('common.enabled')" color="primary" hide-details />
          <v-btn color="primary" :disabled="!form.url" @click="save">{{ t("common.save") }}</v-btn>
        </v-card-text>
      </v-card>
    </v-expand-transition>

    <v-card v-if="tools.length" class="settings-panel mb-4">
      <v-card-title class="text-subtitle-1">{{ t("common.tools") }} ({{ tools.length }})</v-card-title>
      <v-list density="compact">
        <v-list-item
          v-for="tool in tools"
          :key="tool.server_id + ':' + tool.name"
          :title="`${tool.server_name} / ${tool.name}`"
          :subtitle="tool.description || undefined"
        />
      </v-list>
    </v-card>

    <v-list lines="three" class="mcp-server-list bg-transparent">
      <v-list-item
        v-for="server in servers"
        :key="server.id"
        :title="server.name"
        :subtitle="`${server.transport} · ${server.url}`"
        class="mcp-server-item mb-2"
      >
        <template #append>
          <v-btn icon="mdi-connection" variant="text" :title="t('common.test')" @click="test(server.id)" />
          <v-btn icon="mdi-delete-outline" variant="text" @click="remove(server.id)" />
        </template>
        <div class="text-caption text-medium-emphasis">
          {{ server.enabled ? t("common.enabled") : t("common.disabled") }}
          <span v-if="testResult[server.id]"> · {{ testResult[server.id] }}</span>
        </div>
      </v-list-item>
      <v-list-item v-if="!servers.length" :title="t('mcp.empty')" />
    </v-list>
  </v-container>
</template>

<style scoped>
.mcp-page {
  width: 100%;
  max-width: 840px;
  height: 100%;
  margin: 0 auto;
  padding: 20px 20px 40px !important;
  box-sizing: border-box;
  overflow: auto;
}
.settings-panel {
  overflow: hidden;
  border: 0;
  border-radius: 16px !important;
  background: var(--miuix-surface-container) !important;
  box-shadow: none !important;
}
.mcp-server-list {
  display: grid;
  gap: 8px;
}
.mcp-page :deep(.mcp-server-item) {
  min-width: 0;
  border: 0 !important;
  border-radius: 16px !important;
  background: var(--miuix-surface-container);
}
.mcp-page :deep(.mcp-server-item:hover) {
  background: color-mix(in srgb, var(--miuix-surface-container) 94%, var(--miuix-on-container) 6%);
}
.mcp-page :deep(.v-list-item-subtitle) {
  white-space: normal;
  overflow-wrap: anywhere;
}

@media (max-width: 679px) {
  .mcp-page {
    padding-top: 18px !important;
    padding-inline: 14px !important;
  }
  .mcp-header {
    display: grid !important;
    grid-template-columns: 40px minmax(0, 1fr);
    align-items: center !important;
    gap: 8px 10px !important;
    margin-bottom: 14px !important;
  }
  .mcp-title {
    max-width: 100%;
    overflow: hidden;
    font-size: 1.15rem !important;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mcp-desc {
    display: none;
  }
  .mcp-header > .v-btn {
    grid-column: 1 / -1;
    width: 100%;
  }
}
</style>
