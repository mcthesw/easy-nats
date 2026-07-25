<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { SiteLocale } from "../i18n/messages";
import { themeIds, type ThemeId } from "../theme/themes";
import PreferenceMenu from "./PreferenceMenu.vue";

defineProps<{
  locale: SiteLocale;
  theme: ThemeId;
}>();

const emit = defineEmits<{
  localeChange: [locale: SiteLocale];
  themeChange: [theme: ThemeId];
}>();

const { t } = useI18n();

const themeOptions = computed(() =>
  themeIds.map((id) => ({ value: id, label: t(`theme.${id}`) })),
);
const languageOptions = [
  { value: "en", label: "English" },
  { value: "zh", label: "简体中文" },
] as const;
</script>

<template>
  <div class="demo-controls" :aria-label="t('demo.controls')">
    <PreferenceMenu
      :label="t('nav.theme')"
      :model-value="theme"
      icon="theme"
      :options="themeOptions"
      @change="emit('themeChange', $event as ThemeId)"
    />
    <PreferenceMenu
      :label="t('nav.language')"
      :model-value="locale"
      icon="language"
      :options="languageOptions"
      @change="emit('localeChange', $event as SiteLocale)"
    />
  </div>
</template>
