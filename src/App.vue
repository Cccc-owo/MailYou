<template>
  <v-app>
    <router-view />
  </v-app>
</template>

<script setup lang="ts">
import { useThemeController } from '@/composables/useThemeController'

useThemeController()
</script>
