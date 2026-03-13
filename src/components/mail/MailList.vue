<template>
  <div class="mail-list">
    <div class="mail-list__header d-flex align-center justify-space-between ga-3 flex-wrap">
      <div class="d-flex align-center ga-2">
        <v-checkbox
          v-if="messages.length > 0"
          :model-value="allSelected"
          :indeterminate="someSelected && !allSelected"
          hide-details
          density="compact"
          class="mail-list__select-all"
          @update:model-value="onToggleSelectAll"
        />
        <div>
          <div class="text-overline">{{ t('mailList.mailbox') }}</div>
          <div class="text-h5">{{ title }}</div>
        </div>
      </div>
      <div class="d-flex align-center ga-2">
        <v-btn
          v-if="hasUnread"
          size="small"
          variant="tonal"
          prepend-icon="mdi-email-check-outline"
          @click="$emit('mark-all-read')"
        >
          {{ t('mailList.markAllRead') }}
        </v-btn>
        <v-chip v-if="unreadCount > 0" size="small" color="primary">{{ t('mailList.unreadCount', { count: unreadCount }) }}</v-chip>
        <v-chip size="small" color="secondary" variant="tonal">{{ t('mailList.totalCount', { count: messages.length }) }}</v-chip>
      </div>
    </div>

    <!-- Batch toolbar -->
    <div v-if="selectedIds.size > 0" class="mail-list__batch-toolbar d-flex align-center ga-2 flex-wrap mb-3">
      <v-chip size="small" color="primary">{{ t('mailList.selectedCount', { count: selectedIds.size }) }}</v-chip>
      <v-btn size="small" variant="tonal" prepend-icon="mdi-delete-outline" color="error" @click="$emit('batch-delete')">
        {{ t('common.delete') }}
      </v-btn>
      <v-btn size="small" variant="tonal" prepend-icon="mdi-archive-outline" @click="$emit('batch-archive')">
        {{ t('mailList.archive') }}
      </v-btn>
      <v-btn size="small" variant="tonal" prepend-icon="mdi-email-open-outline" @click="$emit('batch-mark-read')">
        {{ t('mailList.markRead') }}
      </v-btn>
      <v-btn size="small" variant="tonal" prepend-icon="mdi-email-outline" @click="$emit('batch-mark-unread')">
        {{ t('mailList.markUnread') }}
      </v-btn>
      <v-menu v-if="moveTargetFolders.length > 0">
        <template #activator="{ props: menuProps }">
          <v-btn size="small" variant="tonal" prepend-icon="mdi-folder-move-outline" v-bind="menuProps">
            {{ t('mailList.moveTo') }}
          </v-btn>
        </template>
        <v-list density="compact">
          <v-list-item
            v-for="folder in moveTargetFolders"
            :key="folder.id"
            :prepend-icon="folder.icon"
            :title="folderDisplayName(folder)"
            @click="$emit('batch-move', folder.id)"
          />
        </v-list>
      </v-menu>
      <v-btn size="small" variant="text" @click="$emit('clear-selection')">{{ t('common.cancel') }}</v-btn>
    </div>

    <v-alert v-if="error" class="mb-4" type="error" variant="tonal">{{ error }}</v-alert>
    <v-progress-linear v-if="isLoading" indeterminate color="primary" />

    <div v-if="!isLoading && messages.length === 0" class="mail-list__empty">
      <v-icon :icon="isSearchResult ? 'mdi-magnify' : 'mdi-inbox-outline'" size="40" class="mb-3" />
      <div class="text-h6 mb-1">{{ isSearchResult ? t('mailList.noSearchResults') : t('mailList.noMessages') }}</div>
      <div class="text-body-2 text-medium-emphasis">
        {{ isSearchResult ? t('mailList.noSearchResultsHint') : t('mailList.emptyFolder') }}
      </div>
    </div>

    <v-virtual-scroll v-else :items="messages" :item-height="100" class="mail-list__items">
      <template #default="{ item: message }">
        <v-list-item
          :key="message.id"
          :active="message.id === selectedMessageId"
          :class="['mail-list__item', { 'mail-list__item--unread': !message.isRead }]"
          rounded="xl"
          @click="$emit('select-message', message.id)"
          @contextmenu="ctxMenu.open($event, message)"
        >
          <template #prepend>
            <v-checkbox
              :model-value="selectedIds.has(message.id)"
              hide-details
              density="compact"
              class="mail-list__checkbox mr-2"
              @click.stop
              @update:model-value="$emit('toggle-selection', message.id)"
            />
            <v-avatar color="primary-container" size="40">{{ message.from.slice(0, 1) }}</v-avatar>
          </template>

          <v-list-item-title class="d-flex align-center justify-space-between ga-4 flex-wrap">
            <span :class="{ 'font-weight-bold': !message.isRead }">{{ message.from }}</span>
            <span class="text-caption text-medium-emphasis">{{ formatDate(message.receivedAt) }}</span>
          </v-list-item-title>
          <v-list-item-subtitle>
            <div :class="['mb-1', message.isRead ? 'font-weight-medium' : 'font-weight-bold']">{{ message.subject }}</div>
            <div class="text-body-2 text-medium-emphasis text-truncate">{{ message.preview }}</div>
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
      </template>
    </v-virtual-scroll>

    <!-- Right-click context menu -->
    <ContextMenu v-model="ctxMenu.isOpen.value" :x="ctxMenu.x.value" :y="ctxMenu.y.value">
      <v-list-item prepend-icon="mdi-reply-outline" :title="t('reader.reply')" @click="$emit('context-reply', ctxMenu.target.value!.id)" />
      <v-list-item prepend-icon="mdi-reply-all-outline" :title="t('reader.replyAll')" @click="$emit('context-reply-all', ctxMenu.target.value!.id)" />
      <v-list-item prepend-icon="mdi-arrow-top-right" :title="t('reader.forward')" @click="$emit('context-forward', ctxMenu.target.value!.id)" />
      <v-divider />
      <v-list-item
        :prepend-icon="ctxMenu.target.value?.isStarred ? 'mdi-star' : 'mdi-star-outline'"
        :title="ctxMenu.target.value?.isStarred ? t('common.unstar') : t('common.star')"
        @click="$emit('toggle-star', ctxMenu.target.value!.id)"
      />
      <v-list-item
        :prepend-icon="ctxMenu.target.value?.isRead ? 'mdi-email-outline' : 'mdi-email-open-outline'"
        :title="ctxMenu.target.value?.isRead ? t('mailList.markUnread') : t('mailList.markRead')"
        @click="$emit('context-toggle-read', ctxMenu.target.value!.id)"
      />
      <v-divider />
      <template v-if="currentFolderKind === 'trash' || currentFolderKind === 'archive'">
        <v-list-item prepend-icon="mdi-inbox-arrow-down" :title="t('reader.restoreToInbox')" @click="$emit('context-restore', ctxMenu.target.value!.id)" />
      </template>
      <template v-else>
        <v-list-item prepend-icon="mdi-archive-outline" :title="t('reader.archive')" @click="$emit('context-archive', ctxMenu.target.value!.id)" />
      </template>
      <v-menu v-if="moveTargetFolders.length > 0" location="end">
        <template #activator="{ props: subMenuProps }">
          <v-list-item prepend-icon="mdi-folder-move-outline" :title="t('mailList.moveTo')" v-bind="subMenuProps" append-icon="mdi-chevron-right" />
        </template>
        <v-list density="compact">
          <v-list-item
            v-for="folder in moveTargetFolders"
            :key="folder.id"
            :prepend-icon="folder.icon"
            :title="folderDisplayName(folder)"
            @click="$emit('context-move', ctxMenu.target.value!.id, folder.id)"
          />
        </v-list>
      </v-menu>
      <v-divider />
      <v-list-item prepend-icon="mdi-delete-outline" :title="t('common.delete')" base-color="error" @click="$emit('context-delete', ctxMenu.target.value!.id)" />
    </ContextMenu>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MailMessage, MailboxFolder } from '@/types/mail'
import ContextMenu from '@/components/ContextMenu.vue'
import { useContextMenu } from '@/composables/useContextMenu'

const { t, locale } = useI18n()
const ctxMenu = useContextMenu<MailMessage>()

const folderDisplayName = (folder: MailboxFolder) =>
  folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name

const props = defineProps<{
  error?: string | null
  isLoading: boolean
  isSearchResult?: boolean
  messages: MailMessage[]
  selectedMessageId: string | null
  selectedIds: Set<string>
  title: string
  folders?: MailboxFolder[]
  currentFolderId?: string | null
  currentFolderKind?: string | null
}>()

const emit = defineEmits<{
  'select-message': [messageId: string]
  'toggle-star': [messageId: string]
  'toggle-selection': [messageId: string]
  'select-all': []
  'clear-selection': []
  'mark-all-read': []
  'batch-delete': []
  'batch-archive': []
  'batch-mark-read': []
  'batch-mark-unread': []
  'batch-move': [folderId: string]
  'context-reply': [messageId: string]
  'context-reply-all': [messageId: string]
  'context-forward': [messageId: string]
  'context-toggle-read': [messageId: string]
  'context-archive': [messageId: string]
  'context-restore': [messageId: string]
  'context-delete': [messageId: string]
  'context-move': [messageId: string, folderId: string]
}>()

const hasUnread = computed(() => props.messages.some((m) => !m.isRead))

const unreadCount = computed(() => props.messages.filter((m) => !m.isRead).length)

const allSelected = computed(() =>
  props.messages.length > 0 && props.messages.every((m) => props.selectedIds.has(m.id)),
)

const someSelected = computed(() => props.selectedIds.size > 0)

const onToggleSelectAll = (val: boolean | null) => {
  if (val) {
    emit('select-all')
  } else {
    emit('clear-selection')
  }
}

const moveTargetFolders = computed(() =>
  (props.folders ?? []).filter(
    (f) => f.id !== props.currentFolderId && f.kind !== 'starred',
  ),
)

const formatDate = (value: string) =>
  new Intl.DateTimeFormat(locale.value, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' }).format(
    new Date(value),
  )
</script>

<style scoped>
.mail-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  padding: 20px 16px 0;
}

.mail-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.mail-list__select-all {
  flex: none;
}

.mail-list__batch-toolbar {
  flex-shrink: 0;
  padding: 8px 12px;
  border-radius: 12px;
  background: rgba(var(--v-theme-primary), 0.08);
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
  padding: 0 0 16px;
  flex: 1;
  min-height: 0;
}

.mail-list__item {
  margin-bottom: 10px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.05);
}

.mail-list__item--unread {
  border-left: 3px solid rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.04);
}

.mail-list__item :deep(.v-list-item__content) {
  min-width: 0;
}

.mail-list__item :deep(.v-list-item-title) {
  white-space: normal;
}

.mail-list__checkbox {
  flex: none;
}

.mail-list__append {
  display: flex;
  align-items: center;
  gap: 10px;
}

@media (max-width: 840px) {
  .mail-list {
    padding: 16px 16px 0;
  }

  .mail-list__header {
    flex-wrap: wrap;
    align-items: flex-start;
  }
}

@media (max-width: 600px) {
  .mail-list {
    padding: 12px 12px 0;
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
