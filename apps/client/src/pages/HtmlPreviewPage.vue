<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { loadHtmlPreview } from "../lib/htmlPreview";

defineOptions({ name: "HtmlPreviewPage" });

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const source = loadHtmlPreview(String(route.params.previewId));
</script>

<template>
  <main class="html-preview-page">
    <header class="html-preview-toolbar">
      <v-btn class="html-preview-back" icon="mdi-arrow-left" variant="text" size="small" :aria-label="t('common.back')" @click="router.back()" />
      <div>
        <h1>{{ t("htmlPreview.title") }}</h1>
        <p>{{ t("htmlPreview.sandboxHint") }}</p>
      </div>
    </header>

    <iframe
      v-if="source !== null"
      class="html-preview-frame"
      :srcdoc="source"
      sandbox="allow-forms allow-modals allow-scripts"
      referrerpolicy="no-referrer"
      :title="t('htmlPreview.frameTitle')"
    />
    <div v-else class="html-preview-empty">{{ t("htmlPreview.unavailable") }}</div>
  </main>
</template>

<style scoped>
.html-preview-page {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: rgb(var(--v-theme-background));
}

.html-preview-toolbar {
  min-height: 64px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 18px;
  border-bottom: 1px solid var(--aerina-border);
  background: rgb(var(--v-theme-surface));
}

.html-preview-toolbar h1 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 650;
  line-height: 1.3;
}

.html-preview-toolbar p {
  margin: 2px 0 0;
  color: var(--aerina-muted);
  font-size: 0.72rem;
}

.html-preview-back {
  display: none;
  flex: 0 0 auto;
}

.html-preview-frame {
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
  border: 0;
  background: #ffffff;
}

.html-preview-empty {
  flex: 1 1 auto;
  display: grid;
  place-items: center;
  padding: 24px;
  color: var(--aerina-muted);
  text-align: center;
}

@media (max-width: 959px) {
  .html-preview-toolbar {
    min-height: 58px;
    padding: 8px 12px;
  }

  .html-preview-back {
    display: inline-flex;
  }
}
</style>
