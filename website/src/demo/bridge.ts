import type { WebHandle } from "../generated/easy_nats";
import type { SiteLocale } from "../i18n/messages";
import { isThemeId, type ThemeId } from "../theme/themes";

export interface DemoPreferences {
  language: SiteLocale;
  theme: ThemeId;
}

export interface DemoController {
  preferences(): DemoPreferences | undefined;
  setLanguage(language: SiteLocale): void;
  setTheme(theme: ThemeId): void;
}

export async function startDemo(
  canvas: HTMLCanvasElement,
  preferences: DemoPreferences,
): Promise<DemoController> {
  const module = await import("../generated/easy_nats");
  await module.default();

  const handle = new module.WebHandle();
  await handle.start(canvas, preferences.language, preferences.theme);

  (
    window as Window & {
      easyNatsDemo?: WebHandle;
    }
  ).easyNatsDemo = handle;

  return {
    preferences() {
      const language = handle.language();
      const theme = handle.theme();
      if ((language !== "en" && language !== "zh") || !isThemeId(theme)) {
        return undefined;
      }
      return { language, theme };
    },
    setLanguage(language) {
      handle.set_language(language);
    },
    setTheme(theme) {
      handle.set_theme(theme);
    },
  };
}
