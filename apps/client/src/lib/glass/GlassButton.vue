<script setup lang="ts">
import { computed } from "vue";

/**
 * Liquid Glass 按钮。基于 .aerina-glass-btn 工具类（lib/glass/glass.css）。
 * 支持 tag / disabled / radius / tint 覆盖，内置按压反馈。
 */
const props = withDefaults(
  defineProps<{
    tag?: string;
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    radius?: string;
    tint?: string;
  }>(),
  { tag: "button", type: "button", disabled: false }
);

const emit = defineEmits<{ click: [event: MouseEvent] }>();

const glassStyle = computed(() => ({
  ...(props.radius ? { "--aerina-glass-radius": props.radius } : null),
  ...(props.tint ? { "--aerina-glass-tint": props.tint } : null),
}));

function onClick(event: MouseEvent) {
  if (props.disabled) return;
  emit("click", event);
}
</script>

<template>
  <component
    :is="tag"
    :type="tag === 'button' ? type : undefined"
    :disabled="tag === 'button' ? disabled : undefined"
    class="aerina-glass-btn"
    :style="glassStyle"
    @click="onClick"
  >
    <slot />
  </component>
</template>