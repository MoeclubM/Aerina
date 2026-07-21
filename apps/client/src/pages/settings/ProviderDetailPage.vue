<script setup lang="ts">
defineOptions({ name: "ProviderDetailPage" });

import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  errMessage,
  type CapabilityTag,
  type ModelInfo,
  type ProviderKind,
} from "../../api";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const providerId = computed(() => String(route.params.providerId || ""));

const provider = ref<Awaited<ReturnType<typeof api.listProviders>>[number] | null>(null);
const models = ref<Awaited<ReturnType<typeof api.listModelPresets>>>([]);
const remoteModels = ref<ModelInfo[]>([]);
const selectedRemote = ref<string[]>([]);
const remoteQuery = ref("");
const showRemote = ref(false);
const showModelForm = ref(false);
const error = ref<string | null>(null);
const savingProvider = ref(false);
const savingModel = ref(false);
const fetchingModels = ref(false);
const addingSelected = ref(false);

const providerForm = ref({
  name: "",
  kind: "open_ai_compatible" as ProviderKind,
  base_url: "",
  api_key: "",
  enabled: true,
});

const modelForm = ref({
  name: "",
  model_name: "",
  temperature: 0.7,
  capabilities: ["text", "streaming"] as CapabilityTag[],
  in_random_pool: true,
});

const kindItems = computed(() =>
  (["open_ai_compatible", "open_ai_responses", "anthropic"] as ProviderKind[]).map((value) => ({
    title: t(`providers.kinds.${value}`),
    value,
  })),
);

const capabilityOptions: CapabilityTag[] = [
  "text",
  "streaming",
  "vision",
  "image_generation",
  "reasoning",
  "tool_calling",
];

const existingModelNames = computed(() => new Set(models.value.map((m) => m.model_name)));

const filteredRemote = computed(() => {
  const q = remoteQuery.value.trim().toLowerCase();
  const list = remoteModels.value;
  if (!q) return list;
  return list.filter(
    (m) =>
      m.model_name.toLowerCase().includes(q) ||
      m.display_name.toLowerCase().includes(q),
  );
});

const selectableRemote = computed(() =>
  filteredRemote.value.filter((m) => !existingModelNames.value.has(m.model_name)),
);

async function refresh() {
  const providers = await api.listProviders();
  provider.value = providers.find((p) => p.id === providerId.value) ?? null;
  if (provider.value) {
    providerForm.value = {
      name: provider.value.name,
      kind: provider.value.kind,
      base_url: provider.value.base_url,
      api_key: "",
      enabled: provider.value.enabled,
    };
  }
  const all = await api.listModelPresets();
  models.value = all.filter((m) => m.provider_id === providerId.value);
  selectedRemote.value = selectedRemote.value.filter(
    (id) => !existingModelNames.value.has(id),
  );
}

async function saveProvider() {
  error.value = null;
  savingProvider.value = true;
  try {
    await api.upsertProvider({
      id: providerId.value,
      name: providerForm.value.name.trim(),
      kind: providerForm.value.kind,
      base_url: providerForm.value.base_url.trim(),
      api_key: providerForm.value.api_key.trim() || undefined,
      enabled: providerForm.value.enabled,
    });
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    savingProvider.value = false;
  }
}

async function fetchRemoteModels() {
  error.value = null;
  fetchingModels.value = true;
  showRemote.value = true;
  showModelForm.value = false;
  try {
    const list = await api.listProviderModels(providerId.value);
    remoteModels.value = list;
    selectedRemote.value = [];
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    fetchingModels.value = false;
  }
}

async function addSelectedRemote() {
  if (!selectedRemote.value.length) return;
  error.value = null;
  addingSelected.value = true;
  try {
    for (const mName of selectedRemote.value) {
      const m = remoteModels.value.find((x) => x.model_name === mName);
      if (!m) continue;
      await api.upsertModelPreset({
        provider_id: providerId.value,
        name: m.display_name || m.model_name,
        model_name: m.model_name,
        capabilities: m.capabilities.length
          ? m.capabilities
          : (["text", "streaming"] as CapabilityTag[]),
        in_random_pool: true,
      });
    }
    selectedRemote.value = [];
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    addingSelected.value = false;
  }
}

function selectAllVisible() {
  const ids = selectableRemote.value.map((m) => m.model_name);
  selectedRemote.value = Array.from(new Set([...selectedRemote.value, ...ids]));
}

function clearSelection() {
  selectedRemote.value = [];
}

function remoteSubtitle(item: ModelInfo) {
  const caps = item.capabilities?.length ? item.capabilities.join(" · ") : "text";
  const ctx = item.context_length ? ` · ${Math.round(item.context_length / 1000)}k` : "";
  const fam = item.family ? ` · ${item.family}` : "";
  return `${item.model_name} · ${caps}${ctx}${fam}`;
}

function isRemoteSelected(modelName: string) {
  return selectedRemote.value.includes(modelName);
}

function toggleRemote(modelName: string) {
  if (existingModelNames.value.has(modelName)) return;
  if (selectedRemote.value.includes(modelName)) {
    selectedRemote.value = selectedRemote.value.filter((id) => id !== modelName);
  } else {
    selectedRemote.value = [...selectedRemote.value, modelName];
  }
}

async function saveModel() {
  error.value = null;
  savingModel.value = true;
  try {
    await api.upsertModelPreset({
      provider_id: providerId.value,
      name: modelForm.value.name.trim() || modelForm.value.model_name.trim(),
      model_name: modelForm.value.model_name.trim(),
      capabilities: modelForm.value.capabilities,
      temperature: modelForm.value.temperature,
      in_random_pool: modelForm.value.in_random_pool,
    });
    showModelForm.value = false;
    modelForm.value = {
      name: "",
      model_name: "",
      temperature: 0.7,
      capabilities: ["text", "streaming"],
      in_random_pool: true,
    };
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    savingModel.value = false;
  }
}

async function removeModel(id: string) {
  await api.deleteModelPreset(id);
  await refresh();
}

onMounted(refresh);
watch(providerId, () => {
  remoteModels.value = [];
  selectedRemote.value = [];
  showRemote.value = false;
  showModelForm.value = false;
  refresh();
});
</script>

<template>
  <div class="settings-page provider-detail-page">
    <div class="settings-page-inner">
      <header class="settings-page-header">
        <button
          type="button"
          class="settings-back-btn"
          :title="t('common.back')"
          @click="router.push('/settings/providers')"
        >
          <v-icon icon="mdi-arrow-left" size="20" />
        </button>
        <div class="settings-page-heading">
          <h1 class="settings-page-title text-truncate">{{ provider?.name || t("providers.title") }}</h1>
          <p class="settings-page-desc">{{ t("providers.detail") }}</p>
        </div>
      </header>

      <v-alert
        v-if="error"
        type="error"
        variant="tonal"
        density="comfortable"
        class="mb-4"
        closable
        :text="error"
        @click:close="error = null"
      />
      <v-alert
        v-if="!provider"
        type="warning"
        variant="tonal"
        density="comfortable"
        class="mb-4"
        :text="t('providers.empty')"
      />

      <template v-if="provider">
        <!-- Provider Settings Card -->
        <section class="settings-panel mb-4">
          <div class="settings-panel-title d-flex align-center justify-space-between pb-2">
            <span>{{ t("providers.title") }}</span>
            <v-chip size="x-small" :color="providerForm.enabled ? 'success' : 'default'" variant="tonal">
              {{ providerForm.enabled ? t("common.enabled") : "已禁用" }}
            </v-chip>
          </div>

          <div class="settings-panel-body">
            <div class="form-grid-2col">
              <div class="field-item">
                <label class="compact-field-label mb-1">{{ t('providers.name') }}</label>
                <input
                  v-model="providerForm.name"
                  type="text"
                  class="settings-input"
                />
              </div>

              <div class="field-item">
                <label class="compact-field-label mb-1">{{ t('providers.kind') }}</label>
                <select
                  v-model="providerForm.kind"
                  class="settings-select"
                >
                  <option v-for="item in kindItems" :key="item.value" :value="item.value">
                    {{ item.title }}
                  </option>
                </select>
              </div>

              <div class="field-item">
                <label class="compact-field-label mb-1">{{ t('providers.baseUrl') }}</label>
                <input
                  v-model="providerForm.base_url"
                  type="url"
                  class="settings-input"
                />
              </div>

              <div class="field-item">
                <label class="compact-field-label mb-1">{{ t('providers.apiKey') }}</label>
                <input
                  v-model="providerForm.api_key"
                  type="password"
                  :placeholder="t('providers.apiKeyPlaceholder')"
                  class="settings-input"
                  autocomplete="off"
                />
              </div>
            </div>
          </div>

          <div class="settings-panel-actions pt-2">
            <v-switch
              v-model="providerForm.enabled"
              color="primary"
              :label="t('common.enabled')"
              hide-details
              density="compact"
              inset
              class="me-auto"
            />
            <v-btn color="primary" size="small" :loading="savingProvider" @click="saveProvider">
              {{ t("common.save") }}
            </v-btn>
          </div>
        </section>

        <!-- Models Toolbar -->
        <div class="provider-models-head mb-3">
          <div class="d-flex align-center ga-2">
            <span class="provider-models-title">{{ t("providers.models") }}</span>
            <v-chip size="x-small" color="primary" variant="tonal">{{ models.length }}</v-chip>
          </div>
          <div class="provider-models-actions">
            <v-btn
              color="primary"
              variant="tonal"
              size="small"
              prepend-icon="mdi-cloud-download-outline"
              :loading="fetchingModels"
              @click="fetchRemoteModels"
            >
              {{ t("providers.fetchModels") }}
            </v-btn>
            <v-btn
              variant="tonal"
              size="small"
              prepend-icon="mdi-plus"
              @click="showModelForm = !showModelForm; showRemote = false"
            >
              {{ t("providers.manualAdd") }}
            </v-btn>
          </div>
        </div>

        <!-- Remote Fetch Card -->
        <v-expand-transition>
          <section v-if="showRemote" class="settings-panel mb-4">
            <div class="settings-panel-title">
              {{ t("providers.remoteModels") }}
            </div>
            <div class="px-4 text-caption text-medium-emphasis mb-2">
              使用当前已保存的 Base URL 与 API Key 请求 /models。
            </div>
            <div class="settings-panel-body pt-0">
              <div class="d-flex flex-wrap align-center ga-2 mb-3">
                <input
                  v-model="remoteQuery"
                  type="search"
                  class="settings-input flex-grow-1"
                  style="min-width: 180px;"
                  :placeholder="t('providers.remoteSearch')"
                />
                <v-btn
                  size="small"
                  variant="text"
                  :disabled="!selectableRemote.length"
                  @click="selectAllVisible"
                >
                  {{ t("providers.selectAllVisible") }}
                </v-btn>
                <v-btn
                  size="small"
                  variant="text"
                  :disabled="!selectedRemote.length"
                  @click="clearSelection"
                >
                  {{ t("providers.clearSelection") }}
                </v-btn>
              </div>

              <v-progress-linear v-if="fetchingModels" indeterminate color="primary" class="mb-3" />

              <v-alert
                v-else-if="!filteredRemote.length"
                type="info"
                variant="tonal"
                density="comfortable"
                :text="t('providers.noRemoteModels')"
              />

              <div v-else class="remote-model-compact-list border rounded-lg">
                <div
                  v-for="item in filteredRemote"
                  :key="item.model_name"
                  class="remote-model-row"
                  :class="{ selected: isRemoteSelected(item.model_name), disabled: existingModelNames.has(item.model_name) }"
                  @click="toggleRemote(item.model_name)"
                >
                  <input
                    type="checkbox"
                    class="settings-checkbox me-2"
                    :checked="isRemoteSelected(item.model_name)"
                    :disabled="existingModelNames.has(item.model_name)"
                    @click.stop
                    @change="toggleRemote(item.model_name)"
                  />
                  <div class="remote-model-info">
                    <span class="remote-model-name">{{ item.display_name || item.model_name }}</span>
                    <span class="remote-model-sub">{{ remoteSubtitle(item) }}</span>
                  </div>
                  <v-chip
                    v-if="existingModelNames.has(item.model_name)"
                    size="x-small"
                    variant="tonal"
                  >
                    已添加
                  </v-chip>
                </div>
              </div>
            </div>
            <div class="settings-panel-actions">
              <span class="text-caption text-medium-emphasis me-auto">
                已选中 {{ selectedRemote.length }} 个模型
              </span>
              <v-btn
                color="primary"
                size="small"
                :loading="addingSelected"
                :disabled="!selectedRemote.length"
                @click="addSelectedRemote"
              >
                {{ t("providers.addSelected") }}
              </v-btn>
            </div>
          </section>
        </v-expand-transition>

        <!-- Manual Add Form Card -->
        <v-expand-transition>
          <section v-if="showModelForm" class="settings-panel mb-4">
            <div class="settings-panel-title">
              {{ t("providers.manualAdd") }}
            </div>
            <div class="settings-panel-body">
              <div class="form-grid-2col mb-3">
                <div class="field-item">
                  <label class="compact-field-label mb-1">{{ t('providers.modelName') }}</label>
                  <input
                    v-model="modelForm.name"
                    type="text"
                    class="settings-input"
                  />
                </div>
                <div class="field-item">
                  <label class="compact-field-label mb-1">{{ t('providers.modelId') }}</label>
                  <input
                    v-model="modelForm.model_name"
                    type="text"
                    placeholder="gpt-4o-mini"
                    class="settings-input"
                  />
                </div>
              </div>
              <div>
                <label class="compact-field-label mb-1 d-block">{{ t('providers.capabilities') }}</label>
                <v-chip-group v-model="modelForm.capabilities" multiple column selected-class="text-primary">
                  <v-chip
                    v-for="cap in capabilityOptions"
                    :key="cap"
                    :value="cap"
                    size="small"
                    filter
                    variant="outlined"
                  >
                    {{ cap }}
                  </v-chip>
                </v-chip-group>
              </div>
            </div>
            <div class="settings-panel-actions">
              <v-btn variant="text" size="small" @click="showModelForm = false">{{ t("common.cancel") }}</v-btn>
              <v-btn
                color="primary"
                size="small"
                :loading="savingModel"
                :disabled="!modelForm.model_name.trim()"
                @click="saveModel"
              >
                {{ t("common.save") }}
              </v-btn>
            </div>
          </section>
        </v-expand-transition>

        <!-- Empty Models State -->
        <section v-if="!models.length" class="settings-panel settings-empty-panel py-8">
          <v-icon icon="mdi-cube-outline" size="36" class="text-medium-emphasis mb-2" />
          <div class="settings-empty-title">{{ t("providers.noModels") }}</div>
          <div class="settings-empty-desc">点击上方从 API 拉取或手动添加模型预设</div>
        </section>

        <!-- High Density Models Grid -->
        <div v-else class="models-grid">
          <div
            v-for="model in models"
            :key="model.id"
            class="model-card"
          >
            <div class="model-card-main">
              <div class="d-flex align-center ga-2">
                <span class="model-card-name">{{ model.name }}</span>
              </div>
              <div class="model-card-sub">
                <code>{{ model.model_name }}</code>
                <span v-if="model.capabilities.length" class="ms-1 font-weight-medium">
                  · {{ model.capabilities.join(" / ") }}
                </span>
              </div>
            </div>
            <div class="model-card-actions">
              <v-btn
                icon="mdi-delete-outline"
                size="x-small"
                variant="text"
                color="error"
                @click="removeModel(model.id)"
              />
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.form-grid-2col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px 16px;
}
.field-item {
  display: flex;
  flex-direction: column;
}

@media (max-width: 600px) {
  .form-grid-2col {
    grid-template-columns: 1fr;
  }
}

.compact-field-label {
  font-size: 0.76rem;
  font-weight: 650;
  color: var(--aerina-muted);
  display: block;
}

.provider-detail-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.settings-page-inner {
  max-width: 840px;
  margin: 0 auto;
  padding: 20px 16px 40px;
}
.settings-page-header {
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr);
  gap: 12px 14px;
  align-items: center;
  margin-bottom: 18px;
}
.settings-back-btn {
  width: 38px;
  height: 38px;
  border: 0;
  border-radius: 10px;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.78);
  cursor: pointer;
  transition: background 0.12s ease, transform 0.1s ease;
}
.settings-back-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}
.settings-back-btn:active {
  transform: scale(0.96);
}
.settings-page-heading { min-width: 0; }
.settings-page-title {
  margin: 0 0 2px;
  font-size: 1.25rem;
  font-weight: 700;
  line-height: 1.25;
}
.settings-page-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.85rem;
  line-height: 1.4;
}

.settings-panel {
  border: 1px solid var(--aerina-border);
  border-radius: 14px;
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  overflow: hidden;
}
.settings-panel-title {
  padding: 14px 16px 0;
  font-size: 0.94rem;
  font-weight: 680;
}
.settings-panel-body {
  padding: 12px 16px 14px;
}
.settings-panel-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 8px 14px 12px;
}
.settings-empty-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}
.settings-empty-title {
  font-weight: 680;
  margin-bottom: 4px;
}
.settings-empty-desc {
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.85rem;
}

.provider-models-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.provider-models-title {
  font-size: 0.96rem;
  font-weight: 680;
}
.provider-models-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.models-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 8px;
}
.model-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid var(--aerina-border);
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  transition: border-color var(--aerina-spring), background var(--aerina-spring);
}
.model-card:hover {
  border-color: rgba(var(--v-theme-primary), 0.3);
  background: var(--aerina-material-heavy);
}
.model-card-main {
  min-width: 0;
  flex: 1;
}
.model-card-name {
  font-size: 0.86rem;
  font-weight: 650;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.model-card-sub {
  font-size: 0.72rem;
  color: var(--aerina-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.remote-model-compact-list {
  max-height: 320px;
  overflow-y: auto;
  background: rgba(var(--v-theme-surface), 0.3);
}
.remote-model-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.05);
  cursor: pointer;
  user-select: none;
}
.remote-model-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.04);
}
.remote-model-row.selected {
  background: rgba(var(--v-theme-primary), 0.08);
}
.remote-model-row.disabled {
  opacity: 0.5;
  cursor: default;
}
.remote-model-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.remote-model-name {
  font-size: 0.82rem;
  font-weight: 600;
}
.remote-model-sub {
  font-size: 0.7rem;
  color: var(--aerina-muted);
}
</style>
