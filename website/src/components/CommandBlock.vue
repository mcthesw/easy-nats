<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  command: string;
}>();

const { t } = useI18n();
const copied = ref(false);
let resetTimer: number | undefined;

async function copyCommand(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.command);
    copied.value = true;
    if (resetTimer !== undefined) {
      window.clearTimeout(resetTimer);
    }
    resetTimer = window.setTimeout(() => {
      copied.value = false;
    }, 1600);
  } catch {
    copied.value = false;
  }
}

onBeforeUnmount(() => {
  if (resetTimer !== undefined) {
    window.clearTimeout(resetTimer);
  }
});
</script>

<template>
  <div class="command-block">
    <button
      type="button"
      :aria-label="copied ? t('install.copied') : t('install.copy')"
      :title="copied ? t('install.copied') : t('install.copy')"
      @click="copyCommand"
    >
      <svg v-if="copied" aria-hidden="true" viewBox="0 0 20 20">
        <path d="m4 10 4 4 8-8" />
      </svg>
      <svg v-else aria-hidden="true" viewBox="0 0 20 20">
        <rect x="6.5" y="6.5" width="9" height="9" rx="1.5" />
        <path d="M5 12.5H4.5A1.5 1.5 0 0 1 3 11V4.5A1.5 1.5 0 0 1 4.5 3H11a1.5 1.5 0 0 1 1.5 1.5V5" />
      </svg>
    </button>
    <pre><code>{{ command }}</code></pre>
  </div>
</template>
