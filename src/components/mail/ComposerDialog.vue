<template>
  <v-dialog :model-value="modelValue" max-width="820" @update:model-value="$emit('update:modelValue', $event)">
    <v-card>
      <v-card-title class="d-flex align-center justify-space-between">
        <span>{{ t('composer.compose') }}</span>
        <v-btn icon="mdi-close" variant="text" @click="$emit('close')" />
      </v-card-title>

      <v-card-text class="composer-dialog__body">
        <v-text-field :model-value="draft.to" :label="t('composer.to')" @update:model-value="$emit('update:draft', { ...draft, to: $event })" />
        <v-text-field :model-value="draft.cc" :label="t('composer.cc')" @update:model-value="$emit('update:draft', { ...draft, cc: $event })" />
        <v-text-field :model-value="draft.bcc" :label="t('composer.bcc')" @update:model-value="$emit('update:draft', { ...draft, bcc: $event })" />
        <v-text-field :model-value="draft.subject" :label="t('composer.subject')" @update:model-value="$emit('update:draft', { ...draft, subject: $event })" />
        <v-textarea
          :model-value="draft.body"
          :label="t('composer.message')"
          rows="12"
          variant="solo-filled"
          @update:model-value="$emit('update:draft', { ...draft, body: $event })"
        />
      </v-card-text>

      <v-card-actions class="justify-space-between px-6 pb-6">
        <v-btn prepend-icon="mdi-content-save-outline" :loading="isSaving" @click="$emit('save')">{{ t('composer.saveDraft') }}</v-btn>
        <v-btn color="primary" prepend-icon="mdi-send-outline" :loading="isSending" @click="$emit('send')">
          {{ t('composer.send') }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { DraftMessage } from '@/types/mail'

const { t } = useI18n()

defineProps<{
  draft: DraftMessage
  isSaving: boolean
  isSending: boolean
  modelValue: boolean
}>()

defineEmits<{
  'update:modelValue': [value: boolean]
  'update:draft': [value: DraftMessage]
  save: []
  send: []
  close: []
}>()
</script>

<style scoped>
.composer-dialog__body {
  display: grid;
  gap: 12px;
}
</style>
