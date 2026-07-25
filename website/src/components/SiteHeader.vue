<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <header class="site-header">
    <div class="header-inner">
      <a class="brand" href="#demo" aria-label="Easy NATS">Easy NATS</a>

      <div class="header-meta">
        <span class="header-descriptor">{{ t("demo.eyebrow") }}</span>
        <a
          class="github-link"
          href="https://github.com/mcthesw/easy-nats"
          target="_blank"
          rel="noreferrer"
        >
          {{ t("nav.github") }} <span aria-hidden="true">↗</span>
        </a>
      </div>
    </div>
  </header>
</template>
