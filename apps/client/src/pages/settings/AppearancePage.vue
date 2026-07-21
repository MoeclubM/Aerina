<script setup lang="ts">
defineOptions({ name: "AppearancePage" });

import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, type Profile } from "../../api";
import { useRoleStore, type RoleAssistant } from "../../stores/roles";
import {
  ACCENT_PRESETS,
  DEFAULT_ACCENT,
  usePreferencesStore,
  type LocaleMode,
  type ThemeMode,
} from "../../stores/preferences";

const { t } = useI18n();
const router = useRouter();
const preferences = usePreferencesStore();
const roleStore = useRoleStore();

const showRoleDialog = ref(false);
const editingRoleId = ref<string | null>(null);
const roleForm = ref({
  name: "",
  icon: "mdi-robot-outline",
  description: "",
  systemPrompt: "",
  temperature: 0.7,
});

function openAddRoleDialog() {
  editingRoleId.value = null;
  roleForm.value = {
    name: "",
    icon: "mdi-robot-outline",
    description: "",
    systemPrompt: "",
    temperature: 0.7,
  };
  showRoleDialog.value = true;
}

function openEditRoleDialog(role: RoleAssistant) {
  if (role.isBuiltin) return;
  editingRoleId.value = role.id;
  roleForm.value = {
    name: role.name,
    icon: role.icon,
    description: role.description,
    systemPrompt: role.systemPrompt,
    temperature: role.temperature,
  };
  showRoleDialog.value = true;
}

function saveRole() {
  const name = roleForm.value.name.trim();
  if (!name) return;
  const role = {
    name,
    icon: roleForm.value.icon.trim() || "mdi-robot-outline",
    description: roleForm.value.description.trim(),
    systemPrompt: roleForm.value.systemPrompt.trim(),
    temperature: roleForm.value.temperature,
  };
  if (editingRoleId.value) roleStore.updateRole(editingRoleId.value, role);
  else roleStore.addRole(role);
  showRoleDialog.value = false;
}

const profile = ref<Profile | null>(null);
const avatarUrl = ref<string | null>(null);
const displayName = ref("");
const savingName = ref(false);
const avatarBusy = ref(false);
const avatarInput = ref<HTMLInputElement | null>(null);
const error = ref("");

const themeItems = computed(() => [
  { title: t("common.system"), value: "system" as ThemeMode },
  { title: t("common.light"), value: "light" as ThemeMode },
  { title: t("common.dark"), value: "dark" as ThemeMode },
]);

const localeItems = computed(() => [
  { title: t("common.system"), value: "system" as LocaleMode },
  { title: "English", value: "en" as LocaleMode },
  { title: "简体中文", value: "zh-CN" as LocaleMode },
]);

const accentPresets = computed(() =>
  ACCENT_PRESETS.map((item) => ({
    ...item,
    title: t(item.labelKey),
  })),
);

const customAccent = ref(preferences.accentColor);
watch(
  () => preferences.accentColor,
  (v) => {
    customAccent.value = v;
  },
);

const initial = computed(() => {
  const name = profile.value?.display_name?.trim() || "?";
  return name.slice(0, 1).toUpperCase();
});

async function loadSession() {
  const session = await api.sessionInfo();
  profile.value = session.profile;
  displayName.value = session.profile.display_name;
  await loadAvatar(session.profile.id);
}

async function loadAvatar(profileId: string) {
  avatarUrl.value = await api.getProfileAvatarDataUrl(profileId);
}

async function saveDisplayName() {
  if (!profile.value) return;
  const name = displayName.value.trim();
  if (!name) {
    error.value = t("profile.nameRequired");
    return;
  }
  savingName.value = true;
  error.value = "";
  try {
    const updated = await api.renameProfile(profile.value.id, name);
    profile.value = updated;
    displayName.value = updated.display_name;
  } catch (e) {
    error.value = String(e);
  } finally {
    savingName.value = false;
  }
}

function pickAvatar() {
  avatarInput.value?.click();
}

async function onAvatarFile(ev: Event) {
  const input = ev.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file || !profile.value) return;
  if (!file.type.startsWith("image/")) {
    error.value = t("profile.avatarImageOnly");
    return;
  }
  avatarBusy.value = true;
  error.value = "";
  try {
    const dataUrl = await readFileAsDataUrl(file);
    const updated = await api.setProfileAvatar(profile.value.id, dataUrl);
    profile.value = updated;
    await loadAvatar(updated.id);
  } catch (e) {
    error.value = String(e);
  } finally {
    avatarBusy.value = false;
  }
}

async function clearAvatar() {
  if (!profile.value) return;
  avatarBusy.value = true;
  error.value = "";
  try {
    const updated = await api.clearProfileAvatar(profile.value.id);
    profile.value = updated;
    avatarUrl.value = null;
  } catch (e) {
    error.value = String(e);
  } finally {
    avatarBusy.value = false;
  }
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error ?? new Error("read failed"));
    reader.readAsDataURL(file);
  });
}

function selectPreset(value: string) {
  preferences.setAccentColor(value);
  customAccent.value = value;
}

function applyCustomAccent() {
  try {
    preferences.setAccentColor(customAccent.value);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

function resetAccent() {
  preferences.setAccentColor(DEFAULT_ACCENT);
  customAccent.value = DEFAULT_ACCENT;
}

onMounted(() => {
  void loadSession();
});
</script>

<template>
  <div class="appearance-page">
    <div class="appearance-inner">
      <header class="appearance-header">
        <v-btn
          class="d-md-none appearance-back"
          icon="mdi-arrow-left"
          variant="text"
          @click="router.push('/settings')"
        />
        <div class="appearance-header-text">
          <h1 class="appearance-title">{{ t("appearance.title") }}</h1>
          <p class="appearance-desc">{{ t("appearance.desc") }}</p>
        </div>
      </header>

      <section class="settings-surface">
        <div class="section-head">
          <div class="section-title">{{ t("profile.current") }}</div>
          <div class="section-desc">{{ t("profile.localHint") }}</div>
        </div>

        <div class="profile-editor">
          <button
            type="button"
            class="avatar-pick"
            :disabled="avatarBusy || !profile"
            :title="t('profile.changeAvatar')"
            @click="pickAvatar"
          >
            <img v-if="avatarUrl" class="avatar-img" :src="avatarUrl" alt="" />
            <span v-else class="avatar-letter">{{ initial }}</span>
            <span class="avatar-overlay">
              <v-icon icon="mdi-camera-outline" size="18" />
            </span>
          </button>
          <input
            ref="avatarInput"
            type="file"
            accept="image/png,image/jpeg,image/webp,image/gif"
            hidden
            @change="onAvatarFile"
          />

          <div class="profile-editor-fields">
            <label class="field-label" for="profile-display-name">{{ t("profile.name") }}</label>
            <input
              id="profile-display-name"
              v-model="displayName"
              class="field-input"
              type="text"
              autocomplete="nickname"
              @keydown.enter.prevent="saveDisplayName"
            />
            <div class="profile-actions">
              <button
                type="button"
                class="btn-primary"
                :disabled="savingName || !displayName.trim()"
                @click="saveDisplayName"
              >
                {{ t("profile.saveName") }}
              </button>
              <button
                type="button"
                class="btn-secondary"
                :disabled="avatarBusy"
                @click="pickAvatar"
              >
                {{ t("profile.changeAvatar") }}
              </button>
              <button
                v-if="avatarUrl"
                type="button"
                class="btn-ghost"
                :disabled="avatarBusy"
                @click="clearAvatar"
              >
                {{ t("profile.removeAvatar") }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="settings-surface">
        <div class="setting-block">
          <div class="section-title">{{ t("appearance.theme") }}</div>
          <div class="segmented" role="radiogroup" :aria-label="t('appearance.theme')">
            <button
              v-for="item in themeItems"
              :key="item.value"
              type="button"
              class="segmented-item"
              role="radio"
              :aria-checked="preferences.themeMode === item.value"
              :class="{ active: preferences.themeMode === item.value }"
              @click="preferences.setThemeMode(item.value)"
            >
              {{ item.title }}
            </button>
          </div>
        </div>

        <div class="setting-block">
          <div class="section-title">{{ t("appearance.accent") }}</div>
          <div class="section-desc">{{ t("appearance.accentHint") }}</div>
          <div class="accent-grid">
            <button
              v-for="item in accentPresets"
              :key="item.id"
              type="button"
              class="accent-swatch"
              :class="{ active: preferences.accentColor.toUpperCase() === item.value.toUpperCase() }"
              :style="{ '--swatch': item.value }"
              :title="item.title"
              :aria-label="item.title"
              @click="selectPreset(item.value)"
            >
              <span class="accent-swatch-fill" />
            </button>
          </div>
          <div class="accent-custom">
            <label class="field-label" for="accent-hex">{{ t("appearance.customAccent") }}</label>
            <div class="accent-custom-row">
              <label class="accent-native-wrap" :title="t('appearance.customAccent')">
                <input
                  class="accent-native"
                  type="color"
                  :value="preferences.accentColor"
                  :aria-label="t('appearance.customAccent')"
                  @input="selectPreset(($event.target as HTMLInputElement).value)"
                />
              </label>
              <input
                id="accent-hex"
                v-model="customAccent"
                class="field-input accent-hex"
                type="text"
                spellcheck="false"
                autocomplete="off"
                @keydown.enter.prevent="applyCustomAccent"
                @blur="applyCustomAccent"
              />
              <button type="button" class="btn-ghost" @click="resetAccent">
                {{ t("appearance.resetAccent") }}
              </button>
            </div>
          </div>
        </div>

        <div class="setting-block">
          <div class="d-flex align-center justify-space-between mb-3">
            <div>
              <div class="section-title">助手角色</div>
              <div class="section-desc">内置角色仅用于展示，你可以新增并编辑自己的系统提示词。</div>
            </div>
            <v-btn color="primary" size="small" prepend-icon="mdi-plus" @click="openAddRoleDialog">
              新增自定义角色
            </v-btn>
          </div>

          <div class="mb-4">
            <label class="field-label">默认角色</label>
            <div class="d-flex align-center ga-2 mt-1">
              <v-select
                :model-value="roleStore.defaultRoleId"
                :items="roleStore.roles"
                item-title="name"
                item-value="id"
                density="compact"
                hide-details
                variant="outlined"
                style="max-width: 320px;"
                @update:model-value="roleStore.setDefaultRole($event)"
              />
              <span class="text-caption text-medium-emphasis">新建默认对话将继承此角色的设定</span>
            </div>
          </div>

          <div class="roles-grid">
            <div
              v-for="role in roleStore.roles"
              :key="role.id"
              class="role-item-card"
              :class="{ active: role.id === roleStore.defaultRoleId }"
            >
              <div class="d-flex align-center justify-space-between mb-1">
                <div class="d-flex align-center ga-2">
                  <v-icon :icon="role.icon" size="20" color="primary" />
                  <span class="font-weight-bold text-subtitle-2">{{ role.name }}</span>
                  <v-chip v-if="role.id === roleStore.defaultRoleId" size="x-small" color="primary">默认</v-chip>
                  <v-chip v-if="role.isBuiltin" size="x-small" variant="tonal">内置</v-chip>
                </div>
                <div class="d-flex align-center ga-1">
                  <v-btn
                    v-if="!role.isBuiltin"
                    icon="mdi-pencil-outline"
                    size="x-small"
                    variant="text"
                    @click="openEditRoleDialog(role)"
                  />
                  <v-btn
                    v-if="!role.isBuiltin"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    color="error"
                    @click="roleStore.deleteRole(role.id)"
                  />
                  <v-btn
                    v-if="role.id !== roleStore.defaultRoleId"
                    size="x-small"
                    variant="tonal"
                    color="primary"
                    @click="roleStore.setDefaultRole(role.id)"
                  >
                    设为默认
                  </v-btn>
                </div>
              </div>
              <div class="text-caption text-medium-emphasis mb-2">{{ role.description }}</div>
              <div class="role-prompt-preview">
                <code>{{ role.systemPrompt }}</code>
              </div>
            </div>
          </div>
        </div>

        <div class="setting-block setting-block-last">
          <div class="section-title">{{ t("appearance.language") }}</div>
          <div class="segmented" role="radiogroup" :aria-label="t('appearance.language')">
            <button
              v-for="item in localeItems"
              :key="item.value"
              type="button"
              class="segmented-item"
              role="radio"
              :aria-checked="preferences.localeMode === item.value"
              :class="{ active: preferences.localeMode === item.value }"
              @click="preferences.setLocaleMode(item.value)"
            >
              {{ item.title }}
            </button>
          </div>
        </div>
      </section>

      <div v-if="error" class="error-banner">{{ error }}</div>
    </div>

    <v-dialog v-model="showRoleDialog" max-width="520">
      <v-card rounded="xl" class="pa-2">
        <v-card-title class="d-flex align-center justify-space-between">
          <span>{{ editingRoleId ? "编辑自定义角色" : "新增自定义角色" }}</span>
          <v-btn icon="mdi-close" variant="text" size="small" @click="showRoleDialog = false" />
        </v-card-title>
        <v-card-text class="d-flex flex-column ga-3">
          <v-text-field v-model="roleForm.name" label="角色名称" placeholder="例如：前端开发专家" />
          <v-text-field v-model="roleForm.icon" label="图标 (MDI Icon)" placeholder="mdi-robot-outline" />
          <v-text-field v-model="roleForm.description" label="角色描述" placeholder="描述此角色的专业领域与技能" />
          <v-textarea v-model="roleForm.systemPrompt" label="系统提示词 (System Prompt)" rows="4" placeholder="例如：You are an expert frontend developer..." />
          <div>
            <div class="d-flex align-center justify-space-between text-caption mb-1">
              <span>随机度 / 采样温度 (Temperature): {{ roleForm.temperature.toFixed(1) }}</span>
            </div>
            <input
              v-model.number="roleForm.temperature"
              type="range"
              min="0"
              max="2"
              step="0.1"
              style="width: 100%;"
            />
          </div>
        </v-card-text>
        <v-card-actions class="px-4 pb-4">
          <v-spacer />
          <v-btn variant="text" @click="showRoleDialog = false">取消</v-btn>
          <v-btn color="primary" :disabled="!roleForm.name.trim()" @click="saveRole">{{ editingRoleId ? "保存修改" : "创建角色" }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.appearance-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.appearance-inner {
  max-width: 720px;
  margin: 0 auto;
  padding: 28px 20px 48px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.appearance-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 4px;
}
.appearance-header-text {
  flex: 1;
  min-width: 0;
}
.appearance-title {
  margin: 0 0 6px;
  font-size: 1.5rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.15;
  color: rgb(var(--v-theme-on-background));
}
.appearance-desc {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.45;
  color: rgba(var(--v-theme-on-surface), 0.58);
}

.settings-surface {
  border-radius: 18px;
  border: 1px solid rgba(var(--v-border-color), 0.4);
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  backdrop-filter: blur(20px) saturate(165%);
  -webkit-backdrop-filter: blur(20px) saturate(165%);
  padding: 18px 18px 16px;
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
}

.section-head {
  margin-bottom: 16px;
}
.section-title {
  font-size: 0.92rem;
  font-weight: 620;
  letter-spacing: -0.01em;
  color: rgb(var(--v-theme-on-surface));
  margin-bottom: 4px;
}
.section-desc {
  font-size: 0.8rem;
  line-height: 1.45;
  color: rgba(var(--v-theme-on-surface), 0.55);
}

.profile-editor {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.profile-editor-fields {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.avatar-pick {
  position: relative;
  width: 72px;
  height: 72px;
  border-radius: 20px;
  border: 0;
  padding: 0;
  overflow: hidden;
  cursor: pointer;
  flex: 0 0 auto;
  background: rgba(var(--v-theme-primary), 0.16);
  color: rgb(var(--v-theme-primary));
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-on-surface), 0.06);
  transition: transform 100ms ease-out;
}
.avatar-pick:active:not(:disabled) {
  transform: scale(0.97);
}
.avatar-pick:disabled {
  opacity: 0.7;
  cursor: default;
}
.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.avatar-letter {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  font-size: 1.5rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.avatar-overlay {
  position: absolute;
  inset: auto 0 0 0;
  height: 26px;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.45);
  color: #fff;
  opacity: 0;
  transition: opacity 0.14s ease;
}
.avatar-pick:hover .avatar-overlay,
.avatar-pick:focus-visible .avatar-overlay {
  opacity: 1;
}

.field-label {
  display: block;
  font-size: 0.78rem;
  font-weight: 560;
  color: rgba(var(--v-theme-on-surface), 0.62);
  margin-bottom: 2px;
}
.field-input {
  width: 100%;
  box-sizing: border-box;
  height: 42px;
  border-radius: 12px;
  border: 1px solid rgba(var(--v-border-color), 0.55);
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgb(var(--v-theme-on-surface));
  padding: 0 12px;
  font: inherit;
  font-size: 0.95rem;
  outline: none;
  transition: border-color 0.14s ease, background 0.14s ease, box-shadow 0.14s ease;
}
.field-input:focus {
  border-color: rgba(var(--v-theme-primary), 0.7);
  box-shadow: 0 0 0 3px rgba(var(--v-theme-primary), 0.18);
  background: rgba(var(--v-theme-on-surface), 0.03);
}

.profile-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;
}

.btn-primary,
.btn-secondary,
.btn-ghost {
  border: 0;
  cursor: pointer;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  letter-spacing: 0;
  border-radius: 10px;
  height: 34px;
  padding: 0 12px;
  transition: transform 100ms ease-out, opacity 0.12s ease, background 0.14s ease;
}
.btn-primary:active:not(:disabled),
.btn-secondary:active:not(:disabled),
.btn-ghost:active:not(:disabled) {
  transform: scale(0.97);
}
.btn-primary:disabled,
.btn-secondary:disabled,
.btn-ghost:disabled {
  opacity: 0.45;
  cursor: default;
}
.btn-primary {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}
.btn-secondary {
  background: rgba(var(--v-theme-primary), 0.14);
  color: rgb(var(--v-theme-primary));
}
.btn-ghost {
  background: transparent;
  color: rgb(var(--v-theme-primary));
  padding-inline: 8px;
}

.setting-block {
  padding-bottom: 20px;
  margin-bottom: 20px;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.35);
}
.setting-block-last {
  padding-bottom: 0;
  margin-bottom: 0;
  border-bottom: 0;
}
.setting-block .section-desc {
  margin-bottom: 12px;
}

/* iOS-like segmented control — active text always on-primary */
.segmented {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  padding: 4px;
  border-radius: 12px;
  background: rgba(var(--v-theme-on-surface), 0.07);
  border: 1px solid rgba(var(--v-border-color), 0.3);
}
.segmented-item {
  min-width: 0;
  height: 36px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.78);
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  letter-spacing: 0;
  cursor: pointer;
  padding: 0 8px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background 0.16s ease, color 0.16s ease, transform 100ms ease-out, box-shadow 0.16s ease;
}
.segmented-item:active {
  transform: scale(0.98);
}
.segmented-item.active {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.14);
}

.accent-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 14px;
}
.accent-swatch {
  width: 34px;
  height: 34px;
  border-radius: 11px;
  border: 0;
  padding: 0;
  cursor: pointer;
  background: transparent;
  transition: transform 100ms ease-out;
}
.accent-swatch:active {
  transform: scale(0.94);
}
.accent-swatch-fill {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: 11px;
  background: var(--swatch);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.16),
    0 1px 2px rgba(0, 0, 0, 0.1);
}
.accent-swatch.active .accent-swatch-fill {
  box-shadow:
    0 0 0 2px rgb(var(--v-theme-surface)),
    0 0 0 4px var(--swatch),
    inset 0 0 0 1px rgba(255, 255, 255, 0.2);
}

.accent-custom-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.accent-native-wrap {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  overflow: hidden;
  flex: 0 0 auto;
  border: 1px solid rgba(var(--v-border-color), 0.45);
  cursor: pointer;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
}
.accent-native {
  width: 150%;
  height: 150%;
  margin: -25%;
  border: 0;
  padding: 0;
  cursor: pointer;
  background: transparent;
}
.accent-hex {
  flex: 1;
  min-width: 0;
  max-width: 160px;
  font-variant-numeric: tabular-nums;
  font-family: ui-monospace, "Cascadia Mono", "SF Mono", Consolas, monospace;
  letter-spacing: 0.02em;
}

.error-banner {
  border-radius: 12px;
  padding: 10px 12px;
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
  font-size: 0.86rem;
}

@media (prefers-reduced-transparency: reduce) {
  .settings-surface {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: rgb(var(--v-theme-surface));
  }
}

@media (max-width: 600px) {
  .appearance-inner {
    padding: 18px 14px 36px;
  }
  .profile-editor {
    flex-direction: column;
    align-items: center;
  }
  .profile-editor-fields {
    width: 100%;
  }
  .profile-actions {
    justify-content: center;
  }
  .segmented-item {
    font-size: 0.78rem;
    padding: 0 4px;
  }
  .accent-custom-row {
    flex-wrap: wrap;
  }
  .accent-hex {
    max-width: none;
  }
}
</style>
