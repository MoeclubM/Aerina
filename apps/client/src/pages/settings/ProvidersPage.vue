<script setup lang="ts">
defineOptions({ name: "ProvidersPage" });

import { computed, onActivated, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, errMessage, type ProviderKind } from "../../api";

const { t } = useI18n();
const router = useRouter();

const providers = ref<Awaited<ReturnType<typeof api.listProviders>>>([]);
const presets = ref<Awaited<ReturnType<typeof api.listModelPresets>>>([]);
const showForm = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const form = ref({
  name: "",
  kind: "open_ai_compatible" as ProviderKind,
  base_url: "https://api.openai.com/v1",
  api_key: "",
});

const kindItems = computed(() =>
  (["open_ai_compatible", "open_ai_responses", "anthropic"] as ProviderKind[]).map((value) => ({
    title: t(`providers.kinds.${value}`),
    value,
  })),
);

const showHeaderAdd = computed(() => providers.value.length > 0 && !showForm.value);

async function refresh() {
  providers.value = await api.listProviders();
  presets.value = await api.listModelPresets();
}

function kindLabel(kind: string) {
  const key = `providers.kinds.${kind}`;
  const label = t(key);
  return label === key ? kind : label;
}

function modelCount(providerId: string) {
  return presets.value.filter((p) => p.provider_id === providerId).length;
}

function resetForm() {
  form.value = {
    name: "",
    kind: "open_ai_compatible",
    base_url: "https://api.openai.com/v1",
    api_key: "",
  };
}

function closeForm() {
  showForm.value = false;
  resetForm();
}

async function save() {
  error.value = null;
  saving.value = true;
  try {
    await api.upsertProvider({
      name: form.value.name.trim() || t("providers.title"),
      kind: form.value.kind,
      base_url: form.value.base_url.trim(),
      api_key: form.value.api_key.trim() || undefined,
    });
    showForm.value = false;
    resetForm();
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    saving.value = false;
  }
}

async function remove(id: string) {
  await api.deleteProvider(id);
  await refresh();
}

onMounted(refresh);
onActivated(refresh);
</script>

<template>
  <div class="settings-page providers-page">
    <div class="settings-page-scroll">
      <div class="settings-page-inner">
        <header
          class="settings-page-header"
          :class="{ 'settings-page-header--with-action': showHeaderAdd }"
        >
          <button
            type="button"
            class="settings-back-btn"
            :title="t('common.back')"
            @click="router.push('/settings')"
          >
            <v-icon icon="mdi-arrow-left" size="20" />
          </button>
          <div class="settings-page-heading">
            <h1 class="settings-page-title">{{ t("providers.title") }}</h1>
            <p class="settings-page-desc">{{ t("providers.desc") }}</p>
          </div>
          <button
            v-if="showHeaderAdd"
            type="button"
            class="settings-primary-btn settings-page-header-action"
            @click="showForm = true"
          >
            <v-icon icon="mdi-plus" size="18" />
            <span>{{ t("providers.add") }}</span>
          </button>
        </header>

        <v-alert
          v-if="error"
          type="error"
          variant="tonal"
          density="comfortable"
          class="providers-alert"
          :text="error"
        />

        <v-expand-transition>
          <section v-if="showForm" class="providers-form-section">
            <div class="settings-form-grid">
              <v-text-field
                v-model="form.name"
                variant="filled"
                density="comfortable"
                hide-details="auto"
                :label="t('providers.name')"
                :placeholder="t('providers.namePlaceholder')"
              />
              <v-select
                v-model="form.kind"
                variant="filled"
                density="comfortable"
                :items="kindItems"
                item-title="title"
                item-value="value"
                :label="t('providers.kind')"
                :hint="t('providers.kindHint')"
                persistent-hint
              />
              <v-text-field
                class="settings-form-span"
                v-model="form.base_url"
                variant="filled"
                density="comfortable"
                hide-details="auto"
                :label="t('providers.baseUrl')"
                :placeholder="t('providers.baseUrlPlaceholder')"
                type="url"
              />
              <v-text-field
                class="settings-form-span"
                v-model="form.api_key"
                variant="filled"
                density="comfortable"
                hide-details="auto"
                :label="t('providers.apiKey')"
                :placeholder="t('providers.apiKeyPlaceholder')"
                type="password"
                autocomplete="off"
              />
            </div>
            <div class="settings-form-actions">
              <button type="button" class="settings-ghost-btn" @click="closeForm">
                {{ t("providers.cancelAdd") }}
              </button>
              <button
                type="button"
                class="settings-primary-btn"
                :disabled="!form.base_url.trim() || saving"
                @click="save"
              >
                {{ t("providers.saveProvider") }}
              </button>
            </div>
          </section>
        </v-expand-transition>

        <section v-if="!providers.length && !showForm" class="settings-empty">
          <div class="settings-empty-title">{{ t("providers.empty") }}</div>
          <div class="settings-empty-desc">{{ t("providers.emptyHint") }}</div>
          <button type="button" class="settings-primary-btn settings-empty-action" @click="showForm = true">
            <v-icon icon="mdi-plus" size="18" />
            <span>{{ t("providers.add") }}</span>
          </button>
        </section>

        <section v-else-if="providers.length" class="provider-list">
          <button
            v-for="provider in providers"
            :key="provider.id"
            type="button"
            class="provider-row"
            @click="router.push(`/settings/providers/${provider.id}`)"
          >
            <span class="provider-row-icon">
              <v-icon icon="mdi-server" size="20" />
            </span>
            <span class="provider-row-main">
              <span class="provider-row-title">{{ provider.name }}</span>
              <span class="provider-row-sub">
                {{ kindLabel(provider.kind) }} · {{ provider.base_url }}
              </span>
            </span>
            <span class="provider-row-meta">
              <span class="provider-row-count">
                {{ t("providers.modelsCount", { n: modelCount(provider.id) }) }}
              </span>
              <span
                class="provider-row-delete"
                role="button"
                :title="t('common.delete')"
                @click.stop="remove(provider.id)"
              >
                <v-icon icon="mdi-delete-outline" size="18" />
              </span>
              <v-icon icon="mdi-chevron-right" size="18" class="provider-row-chevron" />
            </span>
          </button>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.providers-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: transparent;
}
.settings-page-scroll {
  flex: 1 1 auto;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
}
.settings-page-inner {
  width: 100%;
  max-width: 840px;
  margin: 0 auto;
  padding: 24px 20px 32px;
  box-sizing: border-box;
}
.settings-page-header {
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr);
  gap: 12px 14px;
  align-items: start;
  margin-bottom: 22px;
}
.settings-page-header--with-action {
  grid-template-columns: 40px minmax(0, 1fr) auto;
}
.settings-back-btn {
  width: 40px;
  height: 40px;
  border: 0;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.78);
  cursor: pointer;
  transition: background 0.12s ease;
}
.settings-back-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}
.settings-back-btn:active {
  background: rgba(var(--v-theme-on-surface), 0.1);
}
.settings-page-heading {
  min-width: 0;
  padding-top: 2px;
}
.settings-page-title {
  margin: 0 0 4px;
  font-size: 1.35rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.25;
}
.settings-page-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.9rem;
  line-height: 1.45;
}
.settings-page-header-action {
  align-self: center;
}
.settings-primary-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 40px;
  padding: 0 14px;
  border: 0;
  border-radius: 12px;
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  font: inherit;
  font-size: 0.9rem;
  font-weight: 620;
  cursor: pointer;
  white-space: nowrap;
  transition: filter 0.12s ease, opacity 0.12s ease;
}
.settings-primary-btn:hover {
  filter: brightness(1.04);
}
.settings-primary-btn:disabled {
  opacity: 0.5;
  cursor: default;
  filter: none;
}
.settings-ghost-btn {
  min-height: 40px;
  padding: 0 14px;
  border: 0;
  border-radius: 12px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgba(var(--v-theme-on-surface), 0.78);
  font: inherit;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}
.settings-ghost-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.09);
}
.providers-alert {
  margin-bottom: 16px;
}
.providers-form-section {
  margin-bottom: 20px;
}
.settings-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 12px;
}
.settings-form-span {
  grid-column: 1 / -1;
}
.settings-form-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}
.settings-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 48px 12px 24px;
}
.settings-empty-title {
  font-weight: 680;
  font-size: 1rem;
  margin-bottom: 6px;
}
.settings-empty-desc {
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.9rem;
  line-height: 1.5;
  max-width: 360px;
}
.settings-empty-action {
  margin-top: 18px;
}
.provider-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  border-block: 1px solid rgba(var(--v-border-color), 0.22);
  border-radius: 12px;
  overflow: hidden;
}
.provider-row {
  width: 100%;
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  text-align: left;
  border: 0;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.16);
  background: transparent;
  padding: 14px 12px;
  color: inherit;
  cursor: pointer;
  transition: background 0.12s ease;
}
.provider-row:last-child {
  border-bottom: 0;
}
.provider-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.04);
}
.provider-row:active {
  background: rgba(var(--v-theme-on-surface), 0.06);
}
.provider-row-icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  color: rgb(var(--v-theme-primary));
  flex: 0 0 auto;
}
.provider-row-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.provider-row-title {
  font-weight: 650;
  font-size: 0.98rem;
  line-height: 1.3;
}
.provider-row-sub {
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.82rem;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.provider-row-meta {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: 0 0 auto;
}
.provider-row-count {
  font-size: 0.75rem;
  font-weight: 600;
  color: rgba(var(--v-theme-on-surface), 0.52);
  white-space: nowrap;
  padding-inline-end: 4px;
}
.provider-row-delete {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  color: rgba(var(--v-theme-on-surface), 0.48);
  transition: background 0.12s ease, color 0.12s ease;
}
.provider-row-delete:hover {
  background: rgba(var(--v-theme-error), 0.1);
  color: rgb(var(--v-theme-error));
}
.provider-row-chevron {
  color: rgba(var(--v-theme-on-surface), 0.32);
}

@media (max-width: 679px) {
  .settings-page-inner {
    padding: 16px 16px 24px;
  }
  .settings-page-header,
  .settings-page-header--with-action {
    grid-template-columns: 40px minmax(0, 1fr);
    align-items: center;
    margin-bottom: 14px;
  }
  .settings-page-title {
    font-size: 1.15rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .settings-page-desc {
    display: none;
  }
  .settings-page-header-action {
    grid-column: 1 / -1;
    width: 100%;
  }
  .settings-form-grid {
    grid-template-columns: 1fr;
  }
  .settings-form-actions {
    flex-direction: row;
    justify-content: stretch;
  }
  .settings-form-actions .settings-ghost-btn,
  .settings-form-actions .settings-primary-btn {
    flex: 1 1 0;
    min-width: 0;
  }
  .provider-row {
    grid-template-columns: 32px minmax(0, 1fr);
    gap: 10px;
    padding: 12px 10px;
  }
  .provider-row-meta {
    grid-column: 2;
    justify-content: flex-start;
    padding-inline-start: 42px;
    margin-top: -2px;
  }
  .provider-row-chevron {
    margin-inline-start: auto;
  }
}
</style>
