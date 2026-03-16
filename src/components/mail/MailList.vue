<template>
  <div class="mail-list">
    <div class="mail-list__header">
      <div class="d-flex align-center ga-2">
        <v-checkbox
          v-if="threads.length > 0"
          :model-value="allSelected"
          :indeterminate="someSelected && !allSelected"
          hide-details
          density="compact"
          class="mail-list__select-all"
          @update:model-value="onToggleSelectAll"
        />
        <div class="text-subtitle-2">{{ title }}</div>
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
        <v-chip v-if="unreadCount > 0" size="x-small" color="primary" variant="tonal">{{ unreadCount }}</v-chip>
        <span class="text-caption text-medium-emphasis">{{ t('mailList.totalCount', { count: threads.length }) }}</span>
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

    <div v-if="showInlineError" class="mail-list__error" @click="errorDismissed = true">
      <v-icon icon="mdi-alert-circle" size="14" />
      <span class="mail-list__error-text">{{ error }}</span>
      <v-icon icon="mdi-close" size="14" class="mail-list__error-close" />
    </div>
    <v-progress-linear v-if="isLoading" indeterminate color="primary" />

    <div v-if="!isLoading && threads.length === 0" class="mail-list__empty">
      <v-icon :icon="isSearchResult ? 'mdi-magnify' : 'mdi-inbox-outline'" size="40" class="mb-3" />
      <div class="text-h6 mb-1">{{ isSearchResult ? t('mailList.noSearchResults') : t('mailList.noMessages') }}</div>
      <div class="text-body-2 text-medium-emphasis">
        {{ isSearchResult ? t('mailList.noSearchResultsHint') : t('mailList.emptyFolder') }}
      </div>
    </div>

    <v-virtual-scroll v-else :items="flatItems" class="mail-list__items">
      <template #default="{ item }">
        <div v-if="item.type === 'header'" class="mail-list__date-header">
          {{ item.label }}
        </div>
        <v-list-item
          v-else
          :key="item.key"
          :active="item.thread.threadId === selectedThreadId"
          :class="[
            'mail-list__item',
            { 'mail-list__item--unread': item.thread.unreadCount > 0 },
            { 'mail-list__item--checked': selectedIds.has(item.thread.message.id) },
          ]"
          @click="$emit('select-message', item.thread.message.id)"
          @contextmenu="ctxMenu.open($event, item.thread.message)"
        >
          <template #prepend>
            <v-checkbox
              :model-value="selectedIds.has(item.thread.message.id)"
              hide-details
              density="compact"
              class="mail-list__checkbox"
              @click.stop
              @update:model-value="$emit('toggle-selection', item.thread.message.id)"
            />
          </template>

          <div class="mail-list__row1">
            <span class="mail-list__from" :class="{ 'font-weight-bold': item.thread.unreadCount > 0 }">
              {{ item.thread.participants.join(', ') }}
            </span>
            <v-chip
              v-if="item.thread.messageCount > 1"
              size="x-small"
              variant="tonal"
              color="secondary"
            >
              {{ item.thread.messageCount }}
            </v-chip>
            <span class="mail-list__date text-caption text-medium-emphasis">{{ formatDate(item.thread.message.receivedAt) }}</span>
            <v-tooltip :text="item.thread.message.isStarred ? t('common.unstar') : t('common.star')" location="bottom">
              <template #activator="{ props: tip }">
                <v-icon
                  v-bind="tip"
                  :icon="item.thread.message.isStarred ? 'mdi-star' : 'mdi-star-outline'"
                  :color="item.thread.message.isStarred ? 'warning' : undefined"
                  size="18"
                  class="mail-list__star"
                  @click.stop="$emit('toggle-star', item.thread.message.id)"
                />
              </template>
            </v-tooltip>
          </div>
          <div class="mail-list__row2" :class="{ 'font-weight-medium': item.thread.unreadCount > 0 }">{{ item.thread.message.subject }}</div>
          <div class="mail-list__row3 text-medium-emphasis">{{ item.thread.message.preview }}</div>

          <template #append>
            <div class="d-flex align-center ga-2">
              <v-icon v-if="item.thread.message.hasAttachments" icon="mdi-paperclip" size="16" class="text-medium-emphasis" />
              <v-chip v-if="item.thread.unreadCount > 0" size="x-small" color="primary" variant="tonal">
                {{ item.thread.unreadCount }}
              </v-chip>
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
      <template v-if="currentFolderKind === 'trash' || currentFolderKind === 'archive' || currentFolderKind === 'junk'">
        <v-list-item prepend-icon="mdi-inbox-arrow-down" :title="currentFolderKind === 'junk' ? t('reader.notSpam') : t('reader.restoreToInbox')" @click="$emit('context-restore', ctxMenu.target.value!.id)" />
      </template>
      <template v-else>
        <v-list-item prepend-icon="mdi-archive-outline" :title="t('reader.archive')" @click="$emit('context-archive', ctxMenu.target.value!.id)" />
        <v-list-item v-if="!props.isPop3" prepend-icon="mdi-alert-circle-outline" :title="t('reader.markSpam')" @click="$emit('context-mark-spam', ctxMenu.target.value!.id)" />
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
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MailMessage, MailThreadSummary, MailboxFolder } from '@/types/mail'
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
  threads: MailThreadSummary[]
  selectedMessageId: string | null
  selectedThreadId?: string | null
  isPop3?: boolean
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
  'context-mark-spam': [messageId: string]
  'context-restore': [messageId: string]
  'context-delete': [messageId: string]
  'context-move': [messageId: string, folderId: string]
}>()

const errorDismissed = ref(false)
const showInlineError = computed(() => Boolean(props.error) && !errorDismissed.value)

watch(() => props.error, () => {
  errorDismissed.value = false
})

const hasUnread = computed(() => props.threads.some((thread) => thread.unreadCount > 0))

const unreadCount = computed(() => props.threads.reduce((sum, thread) => sum + thread.unreadCount, 0))

const allSelected = computed(() =>
  props.threads.length > 0 && props.threads.every((thread) => props.selectedIds.has(thread.message.id)),
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

type ListItem =
  | { type: 'header'; label: string; key: string }
  | { type: 'thread'; thread: MailThreadSummary; key: string }

const dateGroupLabel = (dateStr: string): string => {
  const date = new Date(dateStr)
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const yesterday = new Date(today)
  yesterday.setDate(yesterday.getDate() - 1)
  const weekAgo = new Date(today)
  weekAgo.setDate(weekAgo.getDate() - 6)

  if (date >= today) return t('mailList.today')
  if (date >= yesterday) return t('mailList.yesterday')
  if (date >= weekAgo) return t('mailList.thisWeek')
  return t('mailList.earlier')
}

const flatItems = computed<ListItem[]>(() => {
  const result: ListItem[] = []
  let lastLabel = ''
  for (const thread of props.threads) {
    const label = dateGroupLabel(thread.message.receivedAt)
    if (label !== lastLabel) {
      lastLabel = label
      result.push({ type: 'header', label, key: `header-${label}` })
    }
    result.push({ type: 'thread', thread, key: thread.threadId })
  }
  return result
})

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
  padding: 12px 12px 0;
}

.mail-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
  flex-shrink: 0;
}

.mail-list__error {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  margin-bottom: 8px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(var(--v-theme-error), 0.1);
  font-size: 0.75rem;
  line-height: 1.5;
  color: rgb(var(--v-theme-error));
  cursor: pointer;
}

.mail-list__error-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-list__error-close {
  flex-shrink: 0;
  opacity: 0.6;
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
  min-height: 200px;
  padding: 24px;
}

.mail-list__items {
  padding: 0 0 12px;
  flex: 1;
  min-height: 0;
}

.mail-list__date-header {
  padding: 8px 12px 2px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.mail-list__item {
  margin-bottom: 1px;
}

.mail-list__item :deep(.v-list-item__content) {
  min-width: 0;
  padding: 4px 0;
}

.mail-list__item--unread {
  border-left: 3px solid rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.04);
}

.mail-list__item--checked {
  background: rgba(var(--v-theme-primary), 0.08);
}

.mail-list__item--checked.mail-list__item--unread {
  background: rgba(var(--v-theme-primary), 0.12);
}

.mail-list__row1 {
  display: flex;
  align-items: center;
  gap: 8px;
  line-height: 1.4;
}

.mail-list__from {
  flex-shrink: 0;
  font-size: 0.8125rem;
  max-width: 50%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-list__date {
  flex-shrink: 0;
  margin-left: auto;
}

.mail-list__star {
  flex-shrink: 0;
  cursor: pointer;
}

.mail-list__row2 {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 0.8125rem;
  line-height: 1.4;
}

.mail-list__row3 {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 0.75rem;
  line-height: 1.4;
}

.mail-list__checkbox {
  flex: none;
}

@media (max-width: 840px) {
  .mail-list {
    padding: 12px 12px 0;
  }

  .mail-list__header {
    flex-wrap: wrap;
    align-items: flex-start;
  }
}
</style>
