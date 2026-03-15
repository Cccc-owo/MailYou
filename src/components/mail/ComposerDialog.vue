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

        <div v-if="draft.attachments.length" class="d-flex flex-wrap align-center ga-2">
          <v-chip
            v-for="(att, index) in draft.attachments"
            :key="index"
            closable
            @click:close="removeAttachment(index)"
          >
            <v-icon start icon="mdi-paperclip" />
            {{ att.fileName }}
          </v-chip>
        </div>
      </v-card-text>

      <v-card-actions class="justify-space-between px-6 pb-6">
        <div class="d-flex align-center ga-2">
          <v-btn prepend-icon="mdi-content-save-outline" :loading="isSaving" @click="$emit('save')">{{ t('composer.saveDraft') }}</v-btn>
          <v-btn prepend-icon="mdi-paperclip" @click="triggerFileInput">{{ t('composer.attach') }}</v-btn>
          <input ref="fileInput" type="file" multiple hidden @change="onFilesSelected" />
        </div>
        <v-btn color="primary" prepend-icon="mdi-send-outline" :loading="isSending" @click="$emit('send')">
          {{ t('composer.send') }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DraftMessage } from '@/types/mail'

const { t } = useI18n()
const fileInput = ref<HTMLInputElement | null>(null)

const props = defineProps<{
  draft: DraftMessage
  isSaving: boolean
  isSending: boolean
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:draft': [value: DraftMessage]
  save: []
  send: []
  close: []
}>()

const triggerFileInput = () => {
  fileInput.value?.click()
}

const onFilesSelected = (event: Event) => {
  const input = event.target as HTMLInputElement
  const files = input.files
  if (!files || files.length === 0) return

  const readers: Promise<{ fileName: string; mimeType: string; dataBase64: string }>[] = []
  for (const file of files) {
    readers.push(
      new Promise((resolve) => {
        const reader = new FileReader()
        reader.onload = () => {
          const result = reader.result as string
          // strip "data:...;base64," prefix
          const base64 = result.split(',')[1] || ''
          resolve({
            fileName: file.name,
            mimeType: file.type || 'application/octet-stream',
            dataBase64: base64,
          })
        }
        reader.readAsDataURL(file)
      }),
    )
  }

  Promise.all(readers).then((newAttachments) => {
    emit('update:draft', {
      ...props.draft,
      attachments: [...props.draft.attachments, ...newAttachments],
    })
  })

  // Reset input so the same file can be re-selected
  input.value = ''
}

const removeAttachment = (index: number) => {
  const updated = [...props.draft.attachments]
  updated.splice(index, 1)
  emit('update:draft', { ...props.draft, attachments: updated })
}
</script>

<style scoped>
.composer-dialog__body {
  display: grid;
  gap: 12px;
}
</style>
