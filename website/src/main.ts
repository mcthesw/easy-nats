import { createApp } from "vue";
import { createI18n } from "vue-i18n";

import App from "./App.vue";
import { detectLocale, messages, type SiteLocale } from "./i18n/messages";
import "./style.css";

const initialLocale: SiteLocale = detectLocale(navigator.languages);
const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: "en",
  messages,
});

createApp(App).use(i18n).mount("#app");
