<template>
  <div class="mail-list">
    <div class="mail-list__header d-flex align-center justify-space-between ga-3 flex-wrap">
      <div>
        <div class="text-overline">Mailbox</div>
        <div class="text-h5">{{ title }}</div>
      </div>
      <v-chip size="small" color="secondary">{{ messages.length }} messages</v-chip>
    </div>

    <v-progress-linear v-if="isLoading" indeterminate color="primary" />

    <div v-if="!isLoading && messages.length === 0" class="mail-list__empty">
      <v-icon icon="mdi-inbox-outline" size="40" class="mb-3" />
      <div class="text-h6 mb-1">No messages yet</div>
      <div class="text-body-2 text-medium-emphasis">This folder is empty for now.</div>
    </div>

    <v-list v-else class="mail-list__items">
      <v-list-item
        v-for="message in messages"
        :key="message.id"
        :active="message.id === selectedMessageId"
        class="mail-list__item"
        rounded="xl"
        @click="$emit('select-message', message.id)"
      >
        <template #prepend>
          <v-avatar color="primary-container" size="40">{{ message.from.slice(0, 1) }}</v-avatar>
        </template>

        <v-list-item-title class="d-flex align-center justify-space-between ga-4 flex-wrap">
          <span :class="{ 'font-weight-bold': !message.isRead }">{{ message.from }}</span>
          <span class="text-caption text-medium-emphasis">{{ formatDate(message.receivedAt) }}</span>
        </v-list-item-title>
        <v-list-item-subtitle>
          <div class="font-weight-medium mb-1">{{ message.subject }}</div>
          <div class="text-body-2 text-medium-emphasis">{{ message.preview }}</div>
        </v-list-item-subtitle>

        <template #append>
          <div class="mail-list__append d-flex align-center ga-2">
            <v-icon
              :icon="message.isStarred ? 'mdi-star' : 'mdi-star-outline'"
              :color="message.isStarred ? 'warning' : undefined"
              @click.stop="$emit('toggle-star', message.id)"
            />
            <v-icon v-if="message.hasAttachments" icon="mdi-paperclip" size="18" />
          </div>
        </template>
      </v-list-item>
    </v-list>
  </div>
</template>

<script setup lang="ts">
import type { MailMessage } from '@/types/mail'

defineProps<{
  isLoading: boolean
  messages: MailMessage[]
  selectedMessageId: string | null
  title: string
}>()

defineEmits<{
  'select-message': [messageId: string]
  'toggle-star': [messageId: string]
}>()

const formatDate = (value: string) =>
  new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' }).format(
    new Date(value),
  )
</script>

<style scoped>
.mail-list {
  padding: 20px 16px;
}

.mail-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.mail-list__empty {
  display: grid;
  place-items: center;
  text-align: center;
  min-height: 240px;
  padding: 24px;
  border: 1px dashed rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 24px;
}

.mail-list__items {
  padding: 0;
}

.mail-list__item {
  margin-bottom: 10px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.05);
}

.mail-list__item :deep(.v-list-item__content) {
  min-width: 0;
}

.mail-list__item :deep(.v-list-item-title) {
  white-space: normal;
}

.mail-list__append {
  display: flex;
  align-items: center;
  gap: 10px;
}

@media (max-width: 840px) {
  .mail-list {
    padding: 16px;
  }

  .mail-list__header {
    flex-wrap: wrap;
    align-items: flex-start;
  }
}

@media (max-width: 600px) {
  .mail-list {
    padding: 12px;
  }

  .mail-list__item :deep(.v-list-item__append) {
    align-self: flex-start;
    margin-inline-start: 8px;
  }

  .mail-list__append {
    padding-top: 4px;
  }
}
</style>
