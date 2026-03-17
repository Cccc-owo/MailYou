<template>
  <v-dialog :model-value="modelValue" max-width="820" @update:model-value="$emit('update:modelValue', $event)">
    <v-card>
      <v-card-title class="d-flex align-center justify-space-between">
        <div class="d-flex align-center ga-3">
          <span>{{ t('composer.compose') }}</span>
          <v-chip size="small" variant="tonal" color="secondary">{{ statusLabel }}</v-chip>
        </div>
        <v-btn icon="mdi-close" variant="text" @click="$emit('close')" />
      </v-card-title>

      <v-card-text class="composer-dialog__body">
        <v-select
          v-if="identityOptions.length > 1"
          :model-value="draft.selectedIdentityId"
          :items="identityOptions"
          :label="t('composer.from', 'From')"
          item-title="label"
          item-value="id"
          density="compact"
          @update:model-value="updateIdentity"
        />

        <v-combobox
          :model-value="parseRecipients(draft.to)"
          :items="suggestions"
          :label="t('composer.to')"
          :loading="isSearching"
          multiple
          chips
          closable-chips
          hide-no-data
          item-title="displayLabel"
          item-value="email"
          @update:search="onSearch"
          @update:model-value="updateField('to', $event)"
        >
          <template #chip="{ item, props: chipProps }">
            <v-chip v-bind="chipProps" :text="chipLabel(item)" />
          </template>
          <template #item="{ item, props: itemProps }">
            <v-list-item v-bind="itemProps">
              <v-list-item-title>{{ item.name }}</v-list-item-title>
              <v-list-item-subtitle>{{ item.email }}</v-list-item-subtitle>
            </v-list-item>
          </template>
        </v-combobox>

        <v-combobox
          :model-value="parseRecipients(draft.cc)"
          :items="suggestions"
          :label="t('composer.cc')"
          :loading="isSearching"
          multiple
          chips
          closable-chips
          hide-no-data
          item-title="displayLabel"
          item-value="email"
          @update:search="onSearch"
          @update:model-value="updateField('cc', $event)"
        >
          <template #chip="{ item, props: chipProps }">
            <v-chip v-bind="chipProps" :text="chipLabel(item)" />
          </template>
          <template #item="{ item, props: itemProps }">
            <v-list-item v-bind="itemProps">
              <v-list-item-title>{{ item.name }}</v-list-item-title>
              <v-list-item-subtitle>{{ item.email }}</v-list-item-subtitle>
            </v-list-item>
          </template>
        </v-combobox>

        <v-combobox
          :model-value="parseRecipients(draft.bcc)"
          :items="suggestions"
          :label="t('composer.bcc')"
          :loading="isSearching"
          multiple
          chips
          closable-chips
          hide-no-data
          item-title="displayLabel"
          item-value="email"
          @update:search="onSearch"
          @update:model-value="updateField('bcc', $event)"
        >
          <template #chip="{ item, props: chipProps }">
            <v-chip v-bind="chipProps" :text="chipLabel(item)" />
          </template>
          <template #item="{ item, props: itemProps }">
            <v-list-item v-bind="itemProps">
              <v-list-item-title>{{ item.name }}</v-list-item-title>
              <v-list-item-subtitle>{{ item.email }}</v-list-item-subtitle>
            </v-list-item>
          </template>
        </v-combobox>

        <v-text-field :model-value="draft.subject" :label="t('composer.subject')" @update:model-value="$emit('update:draft', { ...draft, subject: $event })" />
        <RichTextEditor
          :model-value="draft.body"
          :placeholder="t('composer.message')"
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
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DraftMessage } from '@/types/mail'
import type { Contact } from '@/types/contact'
import { mailRepository } from '@/services/mail'
import { applyIdentitySignature as applyIdentitySignatureToBody } from '@/shared/mail/signature'
import { useAccountsStore } from '@/stores/accounts'
import RichTextEditor from '@/components/mail/RichTextEditor.vue'

const { t } = useI18n()
const accountsStore = useAccountsStore()
const fileInput = ref<HTMLInputElement | null>(null)
const suggestions = ref<(Contact & { email: string; displayLabel: string })[]>([])
const isSearching = ref(false)
let searchTimer: ReturnType<typeof setTimeout> | null = null

const props = defineProps<{
  draft: DraftMessage
  draftStatus: 'local-only' | 'server-saved' | 'server-saved-with-local-changes'
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

const identityOptions = computed(() => {
  const account = accountsStore.accounts.find((account) => account.id === props.draft.accountId)
  return (account?.identities ?? []).map((identity) => ({
    id: identity.id,
    label: identity.name ? `${identity.name} <${identity.email}>` : identity.email,
  }))
})

const statusLabel = computed(() => {
  if (props.draftStatus === 'server-saved') {
    return t('composer.status.serverSaved')
  }
  if (props.draftStatus === 'server-saved-with-local-changes') {
    return t('composer.status.serverSavedWithLocalChanges')
  }
  return t('composer.status.localOnly')
})

// Parse comma-separated string into array for combobox
const parseRecipients = (value: string): string[] => {
  return value
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
}

// Serialize combobox array back to comma string
const serializeRecipients = (items: (string | (Contact & { email: string }))[]): string => {
  return items
    .map((item) => {
      if (typeof item === 'string') return item
      return item.name ? `${item.name} <${item.email}>` : item.email
    })
    .join(', ')
}

const updateField = (field: 'to' | 'cc' | 'bcc', items: (string | (Contact & { email: string }))[]) => {
  emit('update:draft', { ...props.draft, [field]: serializeRecipients(items) })
}

const updateIdentity = (identityId: string | null) => {
  const account = accountsStore.accounts.find((account) => account.id === props.draft.accountId)
  const identity = (account?.identities ?? []).find((candidate) => candidate.id === identityId)
  const currentBody = props.draft.body ?? ''
  emit('update:draft', {
    ...props.draft,
    selectedIdentityId: identityId ?? undefined,
    body: identityId ? applyIdentitySignatureToBody(currentBody, identityId, identity?.signature, true) : currentBody,
  })
}

const chipLabel = (item: string | (Contact & { email: string })) => {
  if (typeof item === 'string') return item
  return item.name || item.email
}

const onSearch = (query: string) => {
  if (searchTimer) clearTimeout(searchTimer)
  if (!query || query.length < 2) {
    suggestions.value = []
    return
  }
  searchTimer = setTimeout(async () => {
    isSearching.value = true
    try {
      const results = await mailRepository.searchContacts(query)
      suggestions.value = results.flatMap((c) =>
        c.emails.map((email) => ({ ...c, email, displayLabel: `${c.name} <${email}>` })),
      )
    } catch {
      suggestions.value = []
    } finally {
      isSearching.value = false
    }
  }, 200)
}

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
