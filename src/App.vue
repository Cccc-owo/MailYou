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
</style>
