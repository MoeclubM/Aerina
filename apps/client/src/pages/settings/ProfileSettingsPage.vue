<script setup lang="ts">
defineOptions({ name: "ProfileSettingsPage" });

import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, type Profile } from "../../api";

const { t } = useI18n();
const router = useRouter();
const profile = ref<Profile | null>(null);
const avatarUrl = ref<string | null>(null);
const displayName = ref("");
const savingName = ref(false);
const avatarBusy = ref(false);
const avatarInput = ref<HTMLInputElement | null>(null);
const error = ref("");

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

onMounted(() => {
  void loadSession();
});
</script>

<template>
  <div class="profile-settings-page">
    <div class="profile-settings-inner">
      <header class="profile-settings-header">
        <button
          type="button"
          class="settings-mobile-back"
          :aria-label="t('common.back')"
          @click="router.push('/settings')"
        >
          <v-icon icon="mdi-arrow-left" size="22" />
        </button>
        <div class="profile-settings-header-text">
          <h1 class="profile-settings-title">{{ t("profileSettings.title") }}</h1>
          <p class="profile-settings-desc">{{ t("profileSettings.desc") }}</p>
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

      <div v-if="error" class="error-banner">{{ error }}</div>
    </div>
  </div>
</template>

<style scoped>
.profile-settings-page {
  width: 100%;
  height: 100%;
  min-width: 0;
  overflow: auto;
  background: transparent;
}
.profile-settings-inner {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 28px 20px 48px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.profile-settings-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 4px;
  min-width: 0;
}
.profile-settings-header-text {
  flex: 1;
  min-width: 0;
}
.profile-settings-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.15;
  color: rgb(var(--v-theme-on-background));
  overflow-wrap: anywhere;
}

.settings-surface {
  min-width: 0;
  border-radius: 18px;
  border: 1px solid rgba(var(--v-border-color), 0.4);
  background: var(--aerina-material);
  backdrop-filter: blur(20px) saturate(165%);
  -webkit-backdrop-filter: blur(20px) saturate(165%);
  padding: 18px 18px 16px;
  box-sizing: border-box;
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
}
.section-head {
  margin-bottom: 16px;
}
.section-title {
  margin-bottom: 4px;
  font-size: 0.92rem;
  font-weight: 620;
  letter-spacing: -0.01em;
  color: rgb(var(--v-theme-on-surface));
}
.section-desc {
  font-size: 0.8rem;
  line-height: 1.45;
  color: rgba(var(--v-theme-on-surface), 0.55);
  overflow-wrap: anywhere;
}

.profile-editor {
  display: flex;
  gap: 16px;
  align-items: flex-start;
  min-width: 0;
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
  margin-bottom: 2px;
  font-size: 0.78rem;
  font-weight: 560;
  color: rgba(var(--v-theme-on-surface), 0.62);
}
.field-input {
  width: 100%;
  min-width: 0;
  height: 42px;
  box-sizing: border-box;
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
  min-width: 0;
}
.btn-primary,
.btn-secondary,
.btn-ghost {
  max-width: 100%;
  border: 0;
  cursor: pointer;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  letter-spacing: 0;
  border-radius: 10px;
  min-height: 34px;
  padding: 7px 12px;
  line-height: 1.2;
  overflow-wrap: anywhere;
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
.error-banner {
  border-radius: 12px;
  padding: 10px 12px;
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
  font-size: 0.86rem;
  overflow-wrap: anywhere;
}

@media (prefers-reduced-transparency: reduce) {
  .settings-surface {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: rgb(var(--v-theme-surface));
  }
}

@media (max-width: 679px) {
  .profile-settings-inner {
    padding: 18px 14px 36px;
    gap: 13px;
  }
  .profile-settings-header {
    align-items: center;
    margin-bottom: 0;
  }
  .profile-settings-title {
    font-size: 1.15rem;
  }
  .settings-surface {
    border-radius: 15px;
    padding: 15px 14px 14px;
  }
}

@media (max-width: 600px) {
  .profile-editor {
    flex-direction: column;
    align-items: center;
  }
  .profile-editor-fields {
    width: 100%;
  }
  .profile-actions {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .btn-primary,
  .btn-secondary,
  .btn-ghost {
    width: 100%;
  }
}
</style>