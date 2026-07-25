export const themeIds = [
  "egui-dark",
  "egui-light",
  "catppuccin-latte",
  "catppuccin-frappe",
  "catppuccin-macchiato",
  "catppuccin-mocha",
] as const;

export type ThemeId = (typeof themeIds)[number];

export function detectTheme(prefersDark: boolean): ThemeId {
  return prefersDark ? "egui-dark" : "egui-light";
}

export function applyDocumentTheme(theme: ThemeId): void {
  document.documentElement.dataset.theme = theme;
}

export function isThemeId(value: string | undefined): value is ThemeId {
  return themeIds.some((theme) => theme === value);
}
