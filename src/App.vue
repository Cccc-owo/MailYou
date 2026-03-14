<template>
  <v-app>
    <router-view />
  </v-app>
</template>

<script setup lang="ts">
import { useThemeController } from '@/composables/useThemeController'

useThemeController()
</script>

<style>
/* Custom scrollbar — thin, theme-aware, applied globally */
*,
*::before,
*::after {
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--v-theme-on-surface), 0.2) transparent;
}

::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(var(--v-theme-on-surface), 0.15);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-on-surface), 0.3);
}

::-webkit-scrollbar-corner {
  background: transparent;
}

/* Tooltip — high-contrast, not theme-dependent */
.v-tooltip > .v-overlay__content {
  background: #1a1a1a !important;
  color: #f0f0f0 !important;
  font-size: 0.75rem;
  padding: 4px 10px;
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  pointer-events: none;
}
</style>
