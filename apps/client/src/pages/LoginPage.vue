<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <img class="login-logo" src="/brand/logo-mark.png" :alt="t('app.name')" />
      <h1 class="login-title">{{ t("app.name") }}</h1>
      <p class="login-subtitle">{{ t("app.subtitle") }}</p>

      <v-btn class="login-btn" color="primary" size="large" rounded="pill" block disabled>
        {{ t("login.signIn") }}
      </v-btn>

      <p class="login-hint">{{ t("login.hint") }}</p>
      <p class="login-soon">{{ t("login.comingSoon") }}</p>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  height: 100%;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgb(var(--v-theme-background));
}
.login-card {
  width: 100%;
  max-width: 360px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
}
/* Logo: no background plate; image centered-cropped */
.login-logo {
  width: 92px;
  height: 92px;
  object-fit: cover;
  object-position: center;
}
.login-title {
  margin: 4px 0 0;
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.02em;
}
.login-subtitle {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 14px;
}
.login-btn {
  margin-top: 8px;
}
.login-hint {
  margin: 8px 0 0;
  font-size: 13px;
  color: rgba(var(--v-theme-on-surface), 0.55);
}
.login-soon {
  margin: 0;
  font-size: 12px;
  color: rgba(var(--v-theme-on-surface), 0.4);
}
</style>
