<template>
  <div v-if="message" class="mail-reader">
    <div class="mail-reader__toolbar d-flex justify-space-between ga-4 flex-wrap">
      <div>
        <div class="text-overline">Conversation</div>
        <div class="text-h4">{{ message.subject }}</div>
      </div>
      <div class="mail-reader__toolbar-actions d-flex flex-wrap ga-2">
        <v-btn prepend-icon="mdi-reply-outline" @click="$emit('reply')">Reply</v-btn>
        <v-btn prepend-icon="mdi-reply-all-outline" @click="$emit('reply-all')">Reply All</v-btn>
        <v-btn prepend-icon="mdi-arrow-top-right" @click="$emit('forward')">Forward</v-btn>
        <v-btn prepend-icon="mdi-email-open-outline" @click="$emit('toggle-read')">
          {{ message.isRead ? 'Mark unread' : 'Mark read' }}
        </v-btn>

        <v-menu v-if="moveTargetFolders.length > 0">
          <template #activator="{ props: menuProps }">
            <v-btn prepend-icon="mdi-folder-move-outline" v-bind="menuProps">Move to</v-btn>
          </template>
          <v-list density="compact">
            <v-list-item
              v-for="folder in moveTargetFolders"
              :key="folder.id"
              :prepend-icon="folder.icon"
              :title="folder.name"
              @click="$emit('move', folder.id)"
            />
          </v-list>
        </v-menu>

        <template v-if="isTrashOrArchive">
          <v-btn prepend-icon="mdi-inbox-arrow-down" @click="$emit('restore')">Restore to Inbox</v-btn>
        </template>
        <template v-else>
          <v-btn prepend-icon="mdi-archive-outline" @click="$emit('archive')">Archive</v-btn>
        </template>

        <v-btn prepend-icon="mdi-delete-outline" color="error" @click="$emit('delete')">Delete</v-btn>
      </div>
    </div>

    <v-card class="mail-reader__message" color="surface">
      <div class="mail-reader__meta d-flex justify-space-between ga-4 flex-wrap">
        <div>
          <div class="text-h6">{{ message.from }}</div>
          <div class="text-body-2 text-medium-emphasis">{{ message.fromEmail }}</div>
        </div>
        <div class="text-body-2 text-medium-emphasis text-sm-left text-md-right">
          <div>{{ formattedDate }}</div>
          <div>To {{ message.to.join(', ') }}</div>
          <div v-if="message.cc.length > 0">Cc {{ message.cc.join(', ') }}</div>
        </div>
      </div>

      <div class="mail-reader__chips d-flex flex-wrap ga-2">
        <v-chip v-for="label in message.labels" :key="label" size="small" color="secondary">{{ label }}</v-chip>
        <v-chip v-if="message.hasAttachments" size="small" color="primary">{{ message.attachments.length }} attachments</v-chip>
      </div>

      <div class="mail-reader__body text-body-1" v-html="sanitizedBody" />

      <v-list v-if="message.attachments.length" class="mail-reader__attachments">
        <v-list-item v-for="attachment in message.attachments" :key="attachment.id" rounded="xl">
          <template #prepend>
            <v-icon icon="mdi-paperclip" />
          </template>
          <v-list-item-title>{{ attachment.fileName }}</v-list-item-title>
          <v-list-item-subtitle>{{ formatSize(attachment.sizeBytes) }}</v-list-item-subtitle>
        </v-list-item>
      </v-list>
    </v-card>
  </div>

  <div v-else class="mail-reader__empty">
    <v-icon :icon="emptyStateIcon" size="48" class="mb-4" />
    <div class="text-h5 mb-2">{{ emptyStateTitle }}</div>
    <div class="text-body-1 text-medium-emphasis">{{ emptyStateDescription }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import DOMPurify from 'dompurify'
import type { MailMessage, MailboxFolder } from '@/types/mail'

// Ensure all links in email bodies open externally and have noopener
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') {
    node.setAttribute('target', '_blank')
    node.setAttribute('rel', 'noopener noreferrer')
  }
})

const props = withDefaults(
  defineProps<{
    hasMessages?: boolean
    hasSearchQuery?: boolean
    message: MailMessage | null
    folders?: MailboxFolder[]
    currentFolderId?: string | null
    currentFolderKind?: string | null
  }>(),
  {
    hasMessages: false,
    hasSearchQuery: false,
    folders: () => [],
    currentFolderId: null,
    currentFolderKind: null,
  },
)

defineEmits<{
  reply: []
  'reply-all': []
  forward: []
  archive: []
  restore: []
  delete: []
  'toggle-read': []
  move: [folderId: string]
}>()

const isTrashOrArchive = computed(() =>
  props.currentFolderKind === 'trash' || props.currentFolderKind === 'archive',
)

const moveTargetFolders = computed(() =>
  props.folders.filter(
    (f) => f.id !== props.currentFolderId && f.kind !== 'starred',
  ),
)

const formattedDate = computed(() => {
  if (!props.message) {
    return ''
  }

  return new Intl.DateTimeFormat('en', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(props.message.receivedAt))
})

const emptyStateTitle = computed(() => {
  if (props.hasSearchQuery && !props.hasMessages) {
    return 'No matching message'
  }

  if (!props.hasMessages) {
    return 'No message selected'
  }

  return 'Select a message'
})

const emptyStateDescription = computed(() => {
  if (props.hasSearchQuery && !props.hasMessages) {
    return 'The current search did not match any cached messages in this folder.'
  }

  if (!props.hasMessages) {
    return 'This folder is empty or no message is currently selected.'
  }

  return 'Choose a message from the list to start reading.'
})

const emptyStateIcon = computed(() => (props.hasSearchQuery && !props.hasMessages ? 'mdi-magnify' : 'mdi-email-outline'))

const sanitizedBody = computed(() => {
  if (!props.message) {
    return ''
  }

  return DOMPurify.sanitize(props.message.body, {
    ALLOWED_TAGS: [
      'p', 'br', 'a', 'img', 'table', 'thead', 'tbody', 'tr', 'td', 'th',
      'div', 'span', 'strong', 'b', 'em', 'i', 'u', 'ul', 'ol', 'li',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'pre', 'code',
      'hr', 'sub', 'sup', 'caption', 'col', 'colgroup', 'dd', 'dl', 'dt',
      'center', 'font', 'small', 'big', 'abbr', 'cite',
    ],
    ALLOWED_ATTR: [
      'href', 'src', 'alt', 'title', 'width', 'height', 'style', 'class',
      'align', 'valign', 'border', 'cellpadding', 'cellspacing', 'bgcolor',
      'color', 'size', 'face', 'target', 'colspan', 'rowspan',
    ],
    ALLOW_DATA_ATTR: false,
  })
})

const formatSize = (value: number) => {
  if (value < 1024) {
    return `${value} B`
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`
  }

  return `${(value / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<style scoped>
.mail-reader {
  padding: 20px;
}

.mail-reader__toolbar {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.mail-reader__toolbar-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.mail-reader__message {
  padding: 24px;
}

.mail-reader__meta {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.mail-reader__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}

.mail-reader__body {
  line-height: 1.7;
}

.mail-reader__attachments {
  margin-top: 20px;
  padding: 0;
}

.mail-reader__empty {
  display: grid;
  place-items: center;
  min-height: 100%;
  text-align: center;
  padding: 32px;
}

@media (max-width: 840px) {
  .mail-reader {
    padding: 16px;
  }

  .mail-reader__toolbar,
  .mail-reader__meta {
    flex-direction: column;
    align-items: flex-start;
  }

  .mail-reader__toolbar-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 600px) {
  .mail-reader {
    padding: 12px;
  }

  .mail-reader__message {
    padding: 16px;
  }

  .mail-reader__toolbar-actions :deep(.v-btn) {
    width: 100%;
    justify-content: center;
  }
}
</style>
