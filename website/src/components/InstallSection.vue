<script setup lang="ts">
import { useI18n } from "vue-i18n";

import CommandBlock from "./CommandBlock.vue";

const { t } = useI18n();
const releases = "https://github.com/mcthesw/easy-nats/releases";

const scoopCommand = `scoop bucket add sworld https://github.com/mcthesw/scoop-bucket
scoop install easy-nats`;
const homebrewCommand = `brew install --cask mcthesw/tap/easy-nats
xattr -dr com.apple.quarantine "/Applications/Easy NATS.app"`;
const aptCommand = `echo "deb [trusted=yes] https://mcthesw.github.io/sworld-apt stable main" | \\
  sudo tee /etc/apt/sources.list.d/mcthesw.list
sudo apt update
sudo apt install easy-nats`;
const flatpakCommand =
  "flatpak install flathub io.github.mcthesw.easy-nats";
</script>

<template>
  <section id="install" class="install-section" aria-labelledby="install-title">
    <div class="section-heading">
      <h2 id="install-title">{{ t("install.title") }}</h2>
    </div>

    <div class="install-grid">
      <article class="install-platform">
        <h3>{{ t("install.windows") }}</h3>
        <h4>Scoop</h4>
        <CommandBlock :command="scoopCommand" />
        <p class="download-links">
          <a :href="releases">{{ t("install.portableZip") }}</a>
        </p>
      </article>

      <article class="install-platform">
        <h3>{{ t("install.macos") }}</h3>
        <h4>Homebrew</h4>
        <CommandBlock :command="homebrewCommand" />
        <p class="download-links">
          <a :href="releases">{{ t("install.dmgTarball") }}</a>
        </p>
      </article>

      <article class="install-platform">
        <h3>{{ t("install.linux") }}</h3>
        <h4>APT</h4>
        <CommandBlock :command="aptCommand" />
        <h4>Flathub</h4>
        <CommandBlock :command="flatpakCommand" />
        <p class="download-links linux-download-links">
          <a href="https://flathub.org/apps/io.github.mcthesw.easy-nats">
            {{ t("install.flathub") }}
          </a>
          <a :href="releases">{{ t("install.releaseFormats") }}</a>
        </p>
      </article>
    </div>
  </section>
</template>
