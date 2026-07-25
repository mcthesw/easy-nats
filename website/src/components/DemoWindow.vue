<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import {
  startDemo,
  type DemoController,
  type DemoPreferences,
} from "../demo/bridge";
import type { SiteLocale } from "../i18n/messages";
import type { ThemeId } from "../theme/themes";

const props = defineProps<{
  locale: SiteLocale;
  theme: ThemeId;
}>();

const emit = defineEmits<{
  preferencesChange: [preferences: DemoPreferences];
}>();

const { t } = useI18n();
const canvas = ref<HTMLCanvasElement>();
const status = ref<"preview" | "loading" | "running" | "failed">("preview");
let controller: DemoController | undefined;
let preferenceTimer: number | undefined;

onMounted(async () => {
  if (!window.matchMedia("(min-width: 900px)").matches || !canvas.value) {
    return;
  }

  status.value = "loading";
  try {
    controller = await startDemo(canvas.value, {
      language: props.locale,
      theme: props.theme,
    });
    controller.setLanguage(props.locale);
    controller.setTheme(props.theme);
    status.value = "running";
    preferenceTimer = window.setInterval(() => {
      const preferences = controller?.preferences();
      if (preferences) {
        emit("preferencesChange", preferences);
      }
    }, 250);
  } catch {
    status.value = "failed";
  }
});

onBeforeUnmount(() => {
  if (preferenceTimer !== undefined) {
    window.clearInterval(preferenceTimer);
  }
});

watch(
  () => props.locale,
  (locale) => controller?.setLanguage(locale),
);

watch(
  () => props.theme,
  (theme) => controller?.setTheme(theme),
);
</script>

<template>
  <div class="demo-window">
    <div class="window-bar">
      <span class="traffic-lights" aria-hidden="true">
        <span class="traffic-light traffic-light-close"></span>
        <span class="traffic-light traffic-light-minimize"></span>
        <span class="traffic-light traffic-light-maximize"></span>
      </span>
      <span class="window-title">Easy NATS</span>
    </div>
    <div class="demo-surface" :aria-label="t('demo.label')">
      <canvas ref="canvas" class="demo-canvas"></canvas>
      <img
        class="demo-preview"
        src="/demo-preview.png"
        :alt="t('demo.previewAlt')"
      />
      <p v-if="status === 'loading'" class="demo-status">
        {{ t("demo.loading") }}
      </p>
      <p v-else-if="status === 'failed'" class="demo-status">
        {{ t("demo.failed") }}
      </p>
    </div>
  </div>
  <p class="preview-caption">{{ t("demo.previewCaption") }}</p>
</template>
