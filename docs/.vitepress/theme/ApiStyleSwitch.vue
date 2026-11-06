<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useData } from "vitepress";

const key = "nanaui-docs-style";
const { lang } = useData();
const label = computed(() => (lang.value.startsWith("zh") ? "写法" : "Style"));
const current = ref<"view" | "rust">("view");

function apply(style: "view" | "rust") {
  document.documentElement.classList.remove("prefer-view", "prefer-rust");
  document.documentElement.classList.add(style === "rust" ? "prefer-rust" : "prefer-view");
  current.value = style;
}

function set(style: "view" | "rust") {
  localStorage.setItem(key, style);
  apply(style);
}

function toggle() {
  set(current.value === "rust" ? "view" : "rust");
}

onMounted(() => {
  current.value = document.documentElement.classList.contains("prefer-rust") ? "rust" : "view";
});
</script>

<template>
  <div class="api-style">
    <div class="api-style-label" id="api-style-label">{{ label }}</div>
    <div class="api-style-row" role="radiogroup" aria-labelledby="api-style-label">
      <button
        type="button"
        class="api-style-name"
        data-style="view"
        role="radio"
        :aria-checked="current === 'view'"
        @click="set('view')"
      >
        view!
      </button>
      <button type="button" class="api-switch" tabindex="-1" aria-hidden="true" @click="toggle">
        <span class="api-switch-check"></span>
      </button>
      <button
        type="button"
        class="api-style-name"
        data-style="rust"
        role="radio"
        :aria-checked="current === 'rust'"
        @click="set('rust')"
      >
        Rust
      </button>
    </div>
  </div>
</template>
