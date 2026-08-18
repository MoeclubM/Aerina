<script setup lang="ts">
import { computed } from "vue";

/**
 * Liquid Glass 可复用表面。
 * 材质取自 --aerina-glass-* 变量（见 lib/glass/glass.css，主题自包含）。
 * 通过自定义属性覆盖 radius / blur / tint，避免重复玻璃样式。
 */
const props = withDefaults(
  defineProps<{
    tag?: string;
    radius?: string;
    blur?: string;
    tint?: string;
  }>(),
  { tag: "div" }
);

const glassStyle = computed(() => ({
  ...(props.radius ? { "--aerina-glass-radius": props.radius } : null),
  ...(props.blur ? { "--aerina-glass-lens": props.blur } : null),
  ...(props.tint ? { "--aerina-glass-tint": props.tint } : null),
}));
</script>

<template>
  <component :is="tag" class="aerina-glass" :style="glassStyle">
    <slot />
  </component>
</template>