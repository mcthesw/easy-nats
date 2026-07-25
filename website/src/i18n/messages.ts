export type SiteLocale = "en" | "zh";

export function detectLocale(languages: readonly string[]): SiteLocale {
  return languages.some((language) =>
    language.toLowerCase().startsWith("zh"),
  )
    ? "zh"
    : "en";
}

export const messages = {
  en: {
    meta: {
      description:
        "Try the Easy NATS live WebAssembly demo and find installation options for Windows, macOS, and Linux.",
    },
    nav: {
      label: "Primary navigation",
      demo: "Demo",
      install: "Install",
      theme: "Theme",
      language: "Language",
      github: "GitHub",
    },
    demo: {
      eyebrow: "Live demo · Powered by WebAssembly",
      controls: "Demo preferences",
      label: "Easy NATS live demo",
      loading: "Loading Easy NATS…",
      failed: "The live demo could not start. Refresh the page to try again.",
      previewAlt: "Easy NATS desktop workspace preview",
      previewCaption:
        "The live demo is available on a larger screen. This is a desktop preview.",
    },
    install: {
      title: "Install",
      windows: "Windows",
      macos: "macOS",
      linux: "Linux",
      portableZip: "Portable ZIP",
      dmgTarball: "DMG or tarball",
      releaseFormats: "DEB / RPM / AppImage / tarball",
      flathub: "Flathub",
      copy: "Copy",
      copied: "Copied",
    },
    footer: {
      license: "MIT licensed",
      source: "Source on GitHub",
    },
    theme: {
      "egui-dark": "egui Dark",
      "egui-light": "egui Light",
      "catppuccin-latte": "Catppuccin Latte",
      "catppuccin-frappe": "Catppuccin Frappé",
      "catppuccin-macchiato": "Catppuccin Macchiato",
      "catppuccin-mocha": "Catppuccin Mocha",
    },
  },
  zh: {
    meta: {
      description:
        "在线体验 Easy NATS WebAssembly 实时演示，并查看 Windows、macOS 与 Linux 安装方式。",
    },
    nav: {
      label: "主要导航",
      demo: "体验",
      install: "安装",
      theme: "主题",
      language: "语言",
      github: "GitHub",
    },
    demo: {
      eyebrow: "实时体验 · 由 WebAssembly 驱动",
      controls: "体验设置",
      label: "Easy NATS 实时体验",
      loading: "正在加载 Easy NATS…",
      failed: "实时体验暂时无法启动，请刷新页面后重试。",
      previewAlt: "Easy NATS 桌面工作区预览",
      previewCaption: "请使用更大的屏幕体验实时演示；当前显示桌面端预览。",
    },
    install: {
      title: "安装",
      windows: "Windows",
      macos: "macOS",
      linux: "Linux",
      portableZip: "便携版 ZIP",
      dmgTarball: "DMG 或 tarball",
      releaseFormats: "DEB / RPM / AppImage / tarball",
      flathub: "Flathub",
      copy: "复制",
      copied: "已复制",
    },
    footer: {
      license: "MIT 许可",
      source: "GitHub 源代码",
    },
    theme: {
      "egui-dark": "egui 深色",
      "egui-light": "egui 浅色",
      "catppuccin-latte": "Catppuccin Latte",
      "catppuccin-frappe": "Catppuccin Frappé",
      "catppuccin-macchiato": "Catppuccin Macchiato",
      "catppuccin-mocha": "Catppuccin Mocha",
    },
  },
} as const;
