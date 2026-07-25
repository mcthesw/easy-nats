<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import DemoControls from "./components/DemoControls.vue";
import DemoWindow from "./components/DemoWindow.vue";
import InstallSection from "./components/InstallSection.vue";
import SiteHeader from "./components/SiteHeader.vue";
import type { DemoPreferences } from "./demo/bridge";
import type { SiteLocale } from "./i18n/messages";
import {
  applyDocumentTheme,
  detectTheme,
  type ThemeId,
} from "./theme/themes";

const { locale: i18nLocale, t } = useI18n();
const locale = ref<SiteLocale>(i18nLocale.value as SiteLocale);
const theme = ref<ThemeId>(
  detectTheme(window.matchMedia("(prefers-color-scheme: dark)").matches),
);

watch(
  locale,
  (value) => {
    i18nLocale.value = value;
    document.documentElement.lang = value === "zh" ? "zh-CN" : "en";
    document.title = "Easy NATS";
    document
      .querySelector('meta[name="description"]')
      ?.setAttribute("content", t("meta.description"));
  },
  { immediate: true },
);

watch(theme, applyDocumentTheme, { immediate: true });

function applyDemoPreferences(preferences: DemoPreferences): void {
  locale.value = preferences.language;
  theme.value = preferences.theme;
}
</script>

<template>
  <SiteHeader />

  <main>
    <section id="demo" class="demo-section" :aria-label="t('demo.label')">
      <DemoControls
        :locale="locale"
        :theme="theme"
        @locale-change="locale = $event"
        @theme-change="theme = $event"
      />
      <DemoWindow
        :locale="locale"
        :theme="theme"
        @preferences-change="applyDemoPreferences"
      />
    </section>

    <InstallSection />
  </main>

  <footer class="site-footer">
    <span>Easy NATS · {{ t("footer.license") }}</span>
    <a
      href="https://github.com/mcthesw/easy-nats"
      target="_blank"
      rel="noreferrer"
    >
      {{ t("footer.source") }} <span aria-hidden="true">↗</span>
    </a>
  </footer>
</template>
