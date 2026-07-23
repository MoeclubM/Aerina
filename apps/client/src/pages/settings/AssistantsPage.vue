<script setup lang="ts">
defineOptions({ name: "AssistantsPage" });

import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useRoleStore, type RoleAssistant } from "../../stores/roles";

const { t } = useI18n();
const router = useRouter();
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
</script>

<template>
  <div class="assistants-page">
    <div class="assistants-inner">
      <header class="assistants-header">
        <v-btn
          class="d-md-none assistants-back"
          icon="mdi-arrow-left"
          variant="text"
          @click="router.push('/settings')"
        />
        <div class="assistants-header-text">
          <h1 class="assistants-title">{{ t("assistants.title") }}</h1>
          <p class="assistants-desc">{{ t("assistants.desc") }}</p>
        </div>
      </header>

      <section class="settings-surface">
        <div class="roles-section-header">
          <div class="roles-section-copy">
            <div class="section-title">{{ t("assistants.management") }}</div>
            <div class="section-desc">{{ t("assistants.managementDesc") }}</div>
          </div>
          <v-btn color="primary" size="small" prepend-icon="mdi-plus" @click="openAddRoleDialog">
            {{ t("assistants.add") }}
          </v-btn>
        </div>

        <div class="assistant-role-local-note">
          <v-icon icon="mdi-cellphone-link" size="16" />
          <span>{{ t("appearance.assistantRoleLocalHint") }}</span>
        </div>

        <div class="default-role-block">
          <label class="field-label">{{ t("assistants.defaultRole") }}</label>
          <div class="default-role-row">
            <v-select
              :model-value="roleStore.defaultRoleId"
              :items="roleStore.roles"
              item-title="name"
              item-value="id"
              density="compact"
              hide-details
              variant="outlined"
              class="default-role-select"
              @update:model-value="roleStore.setDefaultRole($event)"
            />
            <span class="default-role-hint">{{ t("assistants.defaultHint") }}</span>
          </div>
        </div>

        <div class="roles-grid">
          <div
            v-for="role in roleStore.roles"
            :key="role.id"
            class="role-item-card"
            :class="{ active: role.id === roleStore.defaultRoleId }"
          >
            <div class="role-card-head">
              <div class="role-card-identity">
                <v-icon :icon="role.icon" size="20" color="primary" />
                <span class="role-card-name">{{ role.name }}</span>
                <v-chip v-if="role.id === roleStore.defaultRoleId" size="x-small" color="primary">{{ t("assistants.defaultBadge") }}</v-chip>
                <v-chip v-if="role.isBuiltin" size="x-small" variant="tonal">{{ t("assistants.builtinBadge") }}</v-chip>
              </div>
              <div class="role-card-actions">
                <v-btn
                  v-if="!role.isBuiltin"
                  icon="mdi-pencil-outline"
                  size="x-small"
                  variant="text"
                  :aria-label="t('assistants.editAria')"
                  @click="openEditRoleDialog(role)"
                />
                <v-btn
                  v-if="!role.isBuiltin"
                  icon="mdi-delete-outline"
                  size="x-small"
                  variant="text"
                  color="error"
                  :aria-label="t('assistants.deleteAria')"
                  @click="roleStore.deleteRole(role.id)"
                />
                <v-btn
                  v-if="role.id !== roleStore.defaultRoleId"
                  size="x-small"
                  variant="tonal"
                  color="primary"
                  @click="roleStore.setDefaultRole(role.id)"
                >
                  {{ t("assistants.setDefault") }}
                </v-btn>
              </div>
            </div>
            <div class="role-description">{{ role.description }}</div>
            <div class="role-prompt-preview">
              <code>{{ role.systemPrompt }}</code>
            </div>
          </div>
        </div>
      </section>
    </div>

    <v-dialog v-model="showRoleDialog" max-width="520">
      <v-card rounded="xl" class="role-dialog-card pa-2">
        <v-card-title class="role-dialog-title">
          <span>{{ editingRoleId ? t("assistants.editTitle") : t("assistants.addTitle") }}</span>
          <v-btn icon="mdi-close" variant="text" size="small" @click="showRoleDialog = false" />
        </v-card-title>
        <v-card-text class="role-dialog-body">
          <label class="role-form-field">
            <span class="field-label">{{ t("assistants.name") }}</span>
            <input v-model="roleForm.name" class="field-input" type="text" :placeholder="t('assistants.namePlaceholder')" />
          </label>
          <label class="role-form-field">
            <span class="field-label">{{ t("assistants.icon") }}</span>
            <input v-model="roleForm.icon" class="field-input" type="text" placeholder="mdi-robot-outline" />
          </label>
          <label class="role-form-field">
            <span class="field-label">{{ t("assistants.description") }}</span>
            <input v-model="roleForm.description" class="field-input" type="text" :placeholder="t('assistants.descriptionPlaceholder')" />
          </label>
          <label class="role-form-field">
            <span class="field-label">{{ t("assistants.systemPrompt") }}</span>
            <textarea
              v-model="roleForm.systemPrompt"
              class="field-input role-prompt-input"
              rows="4"
              :placeholder="t('assistants.promptPlaceholder')"
            />
          </label>
          <div>
            <div class="temperature-label">
              <span>{{ t("assistants.temperature") }}</span>
              <strong>{{ roleForm.temperature.toFixed(1) }}</strong>
            </div>
            <input v-model.number="roleForm.temperature" class="temperature-range" type="range" min="0" max="2" step="0.1" />
          </div>
        </v-card-text>
        <v-card-actions class="role-dialog-actions">
          <v-spacer />
          <v-btn variant="text" @click="showRoleDialog = false">{{ t("common.cancel") }}</v-btn>
          <v-btn color="primary" :disabled="!roleForm.name.trim()" @click="saveRole">
            {{ editingRoleId ? t("assistants.saveChanges") : t("assistants.create") }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.assistants-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.assistants-inner {
  max-width: 720px;
  margin: 0 auto;
  padding: 28px 20px 48px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.assistants-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 4px;
}
.assistants-header-text {
  flex: 1;
  min-width: 0;
}
.assistants-title {
  margin: 0 0 6px;
  color: rgb(var(--v-theme-on-background));
  font-size: 1.5rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.15;
}
.assistants-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.9rem;
  line-height: 1.45;
}
.settings-surface {
  min-width: 0;
  padding: 18px;
  overflow: hidden;
  border: 1px solid rgba(var(--v-border-color), 0.4);
  border-radius: 18px;
  background: var(--aerina-material);
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
  backdrop-filter: blur(20px) saturate(165%);
  -webkit-backdrop-filter: blur(20px) saturate(165%);
}
.roles-section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 12px;
}
.roles-section-copy {
  min-width: 0;
}
.section-title {
  margin-bottom: 4px;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.92rem;
  font-weight: 620;
  letter-spacing: -0.01em;
}
.section-desc {
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.8rem;
  line-height: 1.45;
}
.assistant-role-local-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 10px;
  border-radius: 11px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  background: rgba(var(--v-theme-primary), 0.07);
  font-size: 0.75rem;
  line-height: 1.45;
}
.assistant-role-local-note .v-icon {
  flex: 0 0 auto;
  margin-top: 1px;
  color: rgb(var(--v-theme-primary));
}
.default-role-block {
  margin-top: 18px;
}
.field-label {
  display: block;
  margin-bottom: 4px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.78rem;
  font-weight: 560;
}
.default-role-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.default-role-select {
  flex: 1;
  min-width: 0;
  max-width: 320px;
}
.default-role-hint {
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.75rem;
  line-height: 1.35;
}
.role-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 5px;
}
.role-card-identity {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.role-card-name {
  min-width: 0;
  overflow: hidden;
  font-size: 0.875rem;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.role-card-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}
.role-description {
  margin-bottom: 8px;
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.75rem;
  line-height: 1.4;
}
.role-dialog-card {
  max-height: calc(100dvh - 32px);
  overflow: hidden;
}
.role-dialog-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.role-dialog-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}
.role-form-field {
  display: grid;
  gap: 6px;
}
.field-input {
  width: 100%;
  box-sizing: border-box;
  height: 42px;
  padding: 0 12px;
  border: 1px solid rgba(var(--v-border-color), 0.55);
  border-radius: 12px;
  outline: none;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgb(var(--v-theme-on-surface));
  font: inherit;
  font-size: 0.95rem;
  transition: border-color 0.14s ease, background 0.14s ease, box-shadow 0.14s ease;
}
.field-input:focus {
  border-color: rgba(var(--v-theme-primary), 0.7);
  background: rgba(var(--v-theme-on-surface), 0.03);
  box-shadow: 0 0 0 3px rgba(var(--v-theme-primary), 0.18);
}
.role-prompt-input {
  min-height: 112px;
  height: auto;
  padding-block: 10px;
  resize: vertical;
  line-height: 1.5;
}
.temperature-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 6px;
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.75rem;
}
.temperature-range {
  width: 100%;
}
.role-dialog-actions {
  padding: 0 16px 16px;
}

@media (prefers-reduced-transparency: reduce) {
  .settings-surface {
    background: rgb(var(--v-theme-surface));
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}

@media (max-width: 679px) {
  .assistants-inner {
    padding: 18px 14px 36px;
    gap: 13px;
  }
  .assistants-header {
    align-items: center;
    margin-bottom: 0;
  }
  .assistants-title {
    display: none;
  }
  .assistants-desc {
    font-size: 0.78rem;
    line-height: 1.4;
  }
  .settings-surface {
    padding: 15px 14px 14px;
    border-radius: 15px;
  }
  .roles-section-header {
    flex-wrap: wrap;
    gap: 10px;
  }
  .roles-section-header > .v-btn {
    width: 100%;
  }
  .default-role-row {
    flex-direction: column;
    align-items: stretch;
  }
  .default-role-select,
  .default-role-hint {
    width: 100%;
    max-width: none;
  }
  .roles-grid {
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
  }
  .role-item-card,
  .role-prompt-preview,
  .role-prompt-preview code {
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
  }
  .role-prompt-preview code {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .role-card-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
  }
  .role-card-identity {
    flex-wrap: wrap;
  }
  .role-card-actions {
    justify-self: end;
  }
  .role-dialog-card {
    max-height: calc(100dvh - 16px);
  }
  .role-dialog-actions {
    padding-inline: 10px;
  }
}
</style>
