<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

export interface PreferenceOption {
  value: string;
  label: string;
}

const props = defineProps<{
  label: string;
  modelValue: string;
  icon: "theme" | "language";
  options: readonly PreferenceOption[];
}>();

const emit = defineEmits<{
  change: [value: string];
}>();

const root = ref<HTMLElement>();
const open = ref(false);

function choose(value: string): void {
  emit("change", value);
  open.value = false;
}

function closeOnOutsideClick(event: PointerEvent): void {
  if (!root.value?.contains(event.target as Node)) {
    open.value = false;
  }
}

function closeOnEscape(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    open.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", closeOnOutsideClick);
  document.addEventListener("keydown", closeOnEscape);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closeOnOutsideClick);
  document.removeEventListener("keydown", closeOnEscape);
});
</script>

<template>
  <div ref="root" class="preference-menu">
    <button
      class="preference-trigger"
      type="button"
      :aria-label="label"
      :aria-expanded="open"
      aria-haspopup="listbox"
      :title="label"
      @click="open = !open"
    >
      <svg
        v-if="icon === 'theme'"
        aria-hidden="true"
        viewBox="0 0 24 24"
      >
        <path
          d="M12 3a9 9 0 1 0 0 18h1.2a1.8 1.8 0 0 0 .5-3.53 1.6 1.6 0 0 1 .45-3.14H17a4 4 0 0 0 4-4C21 6.28 16.97 3 12 3Z"
        />
        <circle cx="7.5" cy="10" r="1" />
        <circle cx="10" cy="6.8" r="1" />
        <circle cx="14.2" cy="6.5" r="1" />
        <circle cx="17.2" cy="9" r="1" />
      </svg>
      <svg v-else aria-hidden="true" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="9" />
        <path d="M3 12h18M12 3c2.4 2.47 3.6 5.47 3.6 9S14.4 18.53 12 21M12 3C9.6 5.47 8.4 8.47 8.4 12S9.6 18.53 12 21" />
      </svg>
    </button>

    <div v-if="open" class="preference-popover" role="listbox" :aria-label="label">
      <button
        v-for="option in options"
        :key="option.value"
        type="button"
        role="option"
        :aria-selected="option.value === modelValue"
        :class="{ selected: option.value === modelValue }"
        @click="choose(option.value)"
      >
        <span>{{ option.label }}</span>
        <svg
          v-if="option.value === modelValue"
          aria-hidden="true"
          viewBox="0 0 20 20"
        >
          <path d="m4 10 4 4 8-8" />
        </svg>
      </button>
    </div>
  </div>
</template>
