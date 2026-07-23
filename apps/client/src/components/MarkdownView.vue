<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { isTauri } from "@tauri-apps/api/core";
import { removeHtmlPreview, saveHtmlPreview } from "../lib/htmlPreview";
import { renderMarkdown } from "../lib/markdown";

const props = defineProps<{
  text: string;
  cacheKey?: string;
  streaming?: boolean;
}>();

const { t } = useI18n();
const router = useRouter();
const html = ref("");
const htmlPreviews = ref<string[]>([]);
let timer = 0;
let lastRendered = "";

function paint(force = false) {
  const text = props.text || "";
  if (!force && text === lastRendered) return;
  lastRendered = text;
  // While streaming, skip cache so incomplete markdown can update.
  const rendered = renderMarkdown(text, props.streaming ? undefined : props.cacheKey, t("chat.previewHtml"));
  html.value = rendered.html;
  htmlPreviews.value = rendered.htmlPreviews;
}

function schedule() {
  if (!props.streaming) {
    if (timer) {
      clearTimeout(timer);
      timer = 0;
    }
    paint(true);
    return;
  }
  if (timer) return;
  timer = window.setTimeout(() => {
    timer = 0;
    paint(true);
  }, 48);
}

async function openHtmlPreview(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const button = target.closest<HTMLButtonElement>("[data-html-preview-index]");
  if (!button) return;

  const previewId = saveHtmlPreview(htmlPreviews.value[Number(button.dataset.htmlPreviewIndex)]);
  const previewUrl = router.resolve({ name: "html-preview", params: { previewId } }).href;

  if (window.matchMedia("(max-width: 679px)").matches) {
    await router.push(previewUrl);
    return;
  }

  if (isTauri()) {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const previewWindow = new WebviewWindow(`html-preview-${previewId}`, {
      url: previewUrl,
      title: t("htmlPreview.title"),
      width: 960,
      height: 720,
      minWidth: 640,
      minHeight: 480,
      resizable: true,
      center: true,
    });
    void previewWindow.once("tauri://error", (error) => {
      removeHtmlPreview(previewId);
      console.error("Failed to create HTML preview window", error.payload);
    });
    return;
  }

  const previewWindow = window.open(previewUrl, `html-preview-${previewId}`, "popup,width=960,height=720");
  if (!previewWindow) {
    removeHtmlPreview(previewId);
    throw new Error("Failed to open HTML preview window");
  }
}

watch(
  () => [props.text, props.cacheKey, props.streaming, t("chat.previewHtml")] as const,
  () => schedule(),
  { immediate: true },
);

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <div class="md-body" :class="{ 'md-stream': streaming }" v-html="html" @click="openHtmlPreview" />
</template>
