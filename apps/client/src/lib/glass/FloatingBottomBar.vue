<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

export interface FloatingBarItem {
  icon: string;
  label: string;
}

type ItemBox = { x: number; y: number; w: number; h: number };

/**
 * SukiSU 风格浮动底栏（Liquid Glass）。
 * - 玻璃胶囊容器 + 顶部高光 + 折射滤镜（backdrop-filter）
 * - 滑动胶囊指示器按每个 tab 的真实布局盒定位（不假设等宽）
 * - 支持点击切换与横向拖拽跟手、释放吸附
 * 材质来自 --aerina-glass-*（lib/glass/glass.css）。
 */
const props = withDefaults(
  defineProps<{
    items: FloatingBarItem[];
    modelValue?: number;
    radius?: string;
    tint?: string;
  }>(),
  { modelValue: 0 }
);

const emit = defineEmits<{ "update:modelValue": [index: number] }>();

const barRef = ref<HTMLElement | null>(null);
const boxes = ref<ItemBox[]>([]);
/** 指示器相对 tab 盒的均匀内缩（保留图标边距） */
const indicatorInset = ref(2);
const motionReady = ref(false);

function clamp(v: number, min: number, max: number) {
  return Math.max(min, Math.min(max, v));
}

const current = ref(clamp(props.modelValue, 0, Math.max(props.items.length - 1, 0)));
const dragIndex = ref<number | null>(null);
const animating = ref(true);

watch(
  () => props.modelValue,
  (v) => {
    if (v != null) current.value = clamp(v, 0, Math.max(props.items.length - 1, 0));
  }
);

const visualIndex = computed(() => dragIndex.value ?? current.value);
const activeIndex = computed(() => Math.round(visualIndex.value));

const barStyle = computed(() => ({
  ...(props.radius ? { "--aerina-glass-radius": props.radius } : null),
  ...(props.tint ? { "--aerina-glass-tint": props.tint } : null),
}));

function lerpBox(index: number): ItemBox | null {
  const b = boxes.value;
  if (!b.length) return null;
  const max = b.length - 1;
  const i = clamp(index, 0, max);
  const lo = Math.floor(i);
  const hi = Math.min(Math.ceil(i), max);
  const t = i - lo;
  const a = b[lo];
  const c = b[hi];
  return {
    x: a.x + (c.x - a.x) * t,
    y: a.y + (c.y - a.y) * t,
    w: a.w + (c.w - a.w) * t,
    h: a.h + (c.h - a.h) * t,
  };
}

const indicatorStyle = computed(() => {
  const rect = lerpBox(visualIndex.value);
  if (!rect) return { opacity: "0" as const };
  const inset = indicatorInset.value;
  return {
    opacity: "1",
    width: `${Math.max(rect.w - inset * 2, 0)}px`,
    height: `${Math.max(rect.h - inset * 2, 0)}px`,
    transform: `translate(${rect.x + inset}px, ${rect.y + inset}px)`,
    transition: animating.value && motionReady.value ? undefined : "none",
  };
});

function select(index: number) {
  const i = clamp(Math.round(index), 0, Math.max(props.items.length - 1, 0));
  current.value = i;
  dragIndex.value = null;
  animating.value = true;
  emit("update:modelValue", i);
}

let dragging = false;
let startIndex = 0;
let startX = 0;

function averagePitch() {
  const b = boxes.value;
  if (b.length < 2) return Math.max(b[0]?.w ?? 1, 1);
  return (b[b.length - 1].x - b[0].x) / (b.length - 1);
}

function onPointerDown(e: PointerEvent) {
  if (!boxes.value.length) return;
  dragging = true;
  startIndex = current.value;
  startX = e.clientX;
  animating.value = false;
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
}

function onPointerMove(e: PointerEvent) {
  if (!dragging) return;
  const delta = (e.clientX - startX) / averagePitch();
  dragIndex.value = clamp(startIndex + delta, 0, Math.max(props.items.length - 1, 0));
}

function onPointerUp() {
  dragging = false;
  if (dragIndex.value != null) select(dragIndex.value);
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
}

function measure() {
  const bar = barRef.value;
  if (!bar) return;
  const barRect = bar.getBoundingClientRect();
  const items = bar.querySelectorAll<HTMLElement>(".aerina-float-bar__item");
  const next = Array.from(items).map((el) => {
    const r = el.getBoundingClientRect();
    return {
      x: r.left - barRect.left,
      y: r.top - barRect.top,
      w: r.width,
      h: r.height,
    };
  });
  const prev = boxes.value;
  const same =
    prev.length === next.length &&
    prev.every((p, i) => p.x === next[i].x && p.y === next[i].y && p.w === next[i].w && p.h === next[i].h);
  if (!same) boxes.value = next;

  if (next.length && !motionReady.value) {
    requestAnimationFrame(() => {
      if (!unmounted) motionReady.value = true;
    });
  }
}

let resizeObserver: ResizeObserver | null = null;
let measureRaf = 0;
let unmounted = false;

function scheduleMeasure() {
  if (measureRaf) cancelAnimationFrame(measureRaf);
  measureRaf = requestAnimationFrame(() => {
    measureRaf = 0;
    measure();
  });
}

watch(
  () => props.items.length,
  () => {
    void nextTick(() => {
      observeLayout();
      scheduleMeasure();
    });
  }
);

function observeLayout() {
  const bar = barRef.value;
  if (!bar || !resizeObserver) return;
  resizeObserver.observe(bar);
  for (const el of bar.querySelectorAll(".aerina-float-bar__item")) {
    resizeObserver.observe(el);
  }
}

onMounted(() => {
  resizeObserver = new ResizeObserver(scheduleMeasure);
  measure();
  observeLayout();
  requestAnimationFrame(() => {
    observeLayout();
    requestAnimationFrame(measure);
  });
  window.addEventListener("resize", scheduleMeasure);
  void document.fonts?.ready.then(() => {
    if (!unmounted) measure();
  });
});
onBeforeUnmount(() => {
  unmounted = true;
  if (measureRaf) cancelAnimationFrame(measureRaf);
  resizeObserver?.disconnect();
  window.removeEventListener("resize", scheduleMeasure);
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
});
</script>

<template>
  <div ref="barRef" class="aerina-float-bar" :style="barStyle" @pointerdown="onPointerDown">
    <div class="aerina-float-bar__indicator" :style="indicatorStyle" />
    <button
      v-for="(item, i) in items"
      :key="item.label"
      class="aerina-float-bar__item"
      :class="{ 'is-active': activeIndex === i }"
      type="button"
      @click="select(i)"
    >
      <slot name="item" :item="item" :active="activeIndex === i">
        <span class="aerina-float-bar__icon"><v-icon :icon="item.icon" size="20" /></span>
        <span class="aerina-float-bar__label">{{ item.label }}</span>
      </slot>
    </button>
  </div>
</template>
