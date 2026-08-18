<script setup lang="ts">
import { ref } from "vue";
import GlassButton from "../lib/glass/GlassButton.vue";
import FloatingBottomBar from "../lib/glass/FloatingBottomBar.vue";

const items = [
  { icon: "mdi-message-text-outline", label: "对话" },
  { icon: "mdi-trophy-outline", label: "排行" },
  { icon: "mdi-account-circle-outline", label: "我的" },
  { icon: "mdi-tune-variant", label: "设置" },
];

const active = ref(0);
const pressed = ref<string | null>(null);
</script>

<template>
  <div class="glass-demo">
    <div class="glass-demo-backdrop" />

    <div class="glass-demo-content">
      <h1>Aerina Liquid Glass</h1>
      <p class="sub">SukiSU 风格可复用玻璃组件库 · 拖动底栏切换</p>

      <div class="row">
        <GlassButton @click="pressed = 'primary'">主要按钮</GlassButton>
        <GlassButton radius="12px" @click="pressed = 'small'">小圆角</GlassButton>
        <GlassButton tint="rgb(90 120 255 / 40%)" @click="pressed = 'tinted'">蓝色调</GlassButton>
        <GlassButton disabled>禁用</GlassButton>
      </div>
      <p v-if="pressed" class="msg">点击了：{{ pressed }}</p>
    </div>

    <FloatingBottomBar v-model="active" :items="items" />
  </div>
</template>

<style scoped>
.glass-demo {
  position: relative;
  height: 100%;
  min-height: 100vh;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}
.glass-demo-backdrop {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(1200px 600px at 12% 8%, rgb(255 120 90 / 55%), transparent 55%),
    radial-gradient(1000px 700px at 88% 18%, rgb(120 90 255 / 55%), transparent 55%),
    radial-gradient(900px 700px at 50% 92%, rgb(70 180 220 / 50%), transparent 60%),
    linear-gradient(160deg, #ffd9a0, #ff9ac2 45%, #7a5cff 90%);
}
.glass-demo-content {
  position: relative;
  z-index: 1;
  text-align: center;
  padding: 0 24px;
}
.glass-demo-content h1 {
  font-size: 2.4rem;
  font-weight: 700;
  margin: 0 0 8px;
  color: #2b1b47;
  text-shadow: 0 2px 18px rgb(255 255 255 / 40%);
}
.sub {
  margin: 0 0 28px;
  color: rgb(60 40 90 / 85%);
  font-weight: 500;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  justify-content: center;
}
.msg {
  margin-top: 18px;
  color: rgb(40 20 70 / 90%);
  font-weight: 600;
}
</style>