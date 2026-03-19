<template>
  <div class="mail-list">
    <div class="mail-list__header">
      <div class="mail-list__header-primary">
        <v-checkbox
          v-if="threads.length > 0"
          :model-value="allSelected"
          :indeterminate="someSelected && !allSelected"
          :disabled="batchBusy"
          hide-details
          density="compact"
          class="mail-list__select-all"
          @update:model-value="onToggleSelectAll"
        />
        <div class="mail-list__title text-subtitle-1">{{ title }}</div>
      </div>
      <div class="mail-list__header-meta">
        <v-btn
          v-if="hasUnread"
          size="small"
          variant="tonal"
          prepend-icon="mdi-email-check-outline"
          :disabled="batchBusy"
          @click="$emit('mark-all-read')"
        >
          {{ t('mailList.markAllRead') }}
        </v-btn>
        <v-chip v-if="unreadCount > 0" size="x-small" color="primary" variant="tonal">{{ unreadCount }}</v-chip>
        <span class="mail-list__total text-caption text-medium-emphasis">{{ t('mailList.totalCount', { count: threads.length }) }}</span>
      </div>
    </div>

    <!-- Batch toolbar -->
    <div
      v-if="selectedIds.size > 0"
      class="mail-list__batch-toolbar mb-3"
      :class="{ 'mail-list__batch-toolbar--busy': batchBusy }"
    >
      <div class="mail-list__batch-toolbar-header">
        <v-chip size="small" color="primary">{{ t('mailList.selectedCount', { count: selectedIds.size }) }}</v-chip>
        <v-btn size="small" variant="text" class="mail-list__batch-cancel" @click="$emit('clear-selection')">
          {{ t('common.cancel') }}
        </v-btn>
      </div>

      <div class="mail-list__batch-actions">
        <v-tooltip :text="t('common.delete')" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              size="small"
              variant="tonal"
              icon="mdi-delete-outline"
              color="error"
              :disabled="batchBusy"
              :aria-label="t('common.delete')"
              @click="$emit('batch-delete')"
            />
          </template>
        </v-tooltip>
        <v-tooltip :text="t('mailList.archive')" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              size="small"
              variant="tonal"
              icon="mdi-archive-outline"
              :disabled="batchBusy"
              :aria-label="t('mailList.archive')"
              @click="$emit('batch-archive')"
            />
          </template>
        </v-tooltip>
        <v-tooltip :text="t('mailList.markRead')" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              size="small"
              variant="tonal"
              icon="mdi-email-open-outline"
              :disabled="batchBusy"
              :aria-label="t('mailList.markRead')"
              @click="$emit('batch-mark-read')"
            />
          </template>
        </v-tooltip>
        <v-tooltip :text="t('mailList.markUnread')" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              size="small"
              variant="tonal"
              icon="mdi-email-outline"
              :disabled="batchBusy"
              :aria-label="t('mailList.markUnread')"
              @click="$emit('batch-mark-unread')"
            />
          </template>
        </v-tooltip>
        <v-menu v-if="moveTargetFolders.length > 0">
          <template #activator="{ props: menuProps }">
            <v-tooltip :text="t('mailList.moveTo')" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="{ ...menuProps, ...tooltipProps }"
                  size="small"
                  variant="tonal"
                  icon="mdi-folder-move-outline"
                  :disabled="batchBusy"
                  :aria-label="t('mailList.moveTo')"
                />
              </template>
            </v-tooltip>
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
        <v-tooltip v-if="!props.isPop3" :text="t('labels.manageTitle')" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              size="small"
              variant="tonal"
              icon="mdi-label-multiple-outline"
              :disabled="batchBusy"
              :aria-label="t('labels.manageTitle')"
              @click="$emit('batch-manage-labels')"
            />
          </template>
        </v-tooltip>
      </div>
    </div>

    <div v-if="showInlineError" class="mail-list__error" @click="errorDismissed = true">
      <v-icon icon="mdi-alert-circle" size="14" />
      <span class="mail-list__error-text">{{ error }}</span>
      <v-icon icon="mdi-close" size="14" class="mail-list__error-close" />
    </div>
    <v-progress-linear v-if="isLoading" indeterminate color="primary" />

    <EmptyState
      v-if="!isLoading && threads.length === 0"
      :icon="isSearchResult ? 'mdi-magnify' : 'mdi-inbox-outline'"
      :title="isSearchResult ? t('mailList.noSearchResults') : t('mailList.noMessages')"
      :description="isSearchResult ? t('mailList.noSearchResultsHint') : t('mailList.emptyFolder')"
    />

    <v-virtual-scroll
      v-else
      :items="flatItems"
      item-key="key"
      :item-height="106"
      class="mail-list__items"
    >
      <template #default="{ item }">
        <div v-if="item.type === 'header'" :key="item.key" class="mail-list__date-header">
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
            { 'mail-list__item--pending': item.thread.message.pendingRead },
          ]"
          @mousedown="onItemPointerDown($event)"
          @click="onSelectMessage(item.thread.message.id, $event)"
          @contextmenu="ctxMenu.open($event, item.thread.message)"
        >
          <template #prepend>
            <div class="mail-list__lead">
              <v-icon
                :icon="item.thread.message.hasAttachments ? 'mdi-paperclip' : 'mdi-paperclip-off'"
                size="15"
                class="mail-list__lead-icon text-medium-emphasis"
              />
              <v-checkbox
                :model-value="selectedIds.has(item.thread.message.id)"
                :disabled="batchBusy"
                hide-details
                density="compact"
                class="mail-list__checkbox"
                @mousedown.stop="onItemPointerDown($event)"
                @click.stop.prevent="onToggleSelection(item.thread.message.id, $event)"
              />
              <v-tooltip :text="item.thread.message.isStarred ? t('common.unstar') : t('common.star')" location="bottom">
                <template #activator="{ props: tip }">
                  <v-icon
                    v-bind="tip"
                    :icon="item.thread.message.isStarred ? 'mdi-star' : 'mdi-star-outline'"
                    :color="item.thread.message.isStarred ? 'warning' : undefined"
                    size="18"
                    :class="['mail-list__star', { 'mail-list__star--pending': item.thread.message.pendingStar }]"
                    @click.stop="$emit('toggle-star', item.thread.message.id)"
                  />
                </template>
              </v-tooltip>
            </div>
          </template>

          <div class="mail-list__content">
            <div class="mail-list__row1">
              <div class="mail-list__from-group">
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
              </div>
              <span
                class="mail-list__date text-caption text-medium-emphasis"
                :title="formatDate(item.thread.message.receivedAt, 'full')"
              >
                <span class="mail-list__date-full">{{ formatDate(item.thread.message.receivedAt, 'full') }}</span>
                <span class="mail-list__date-compact">{{ formatDate(item.thread.message.receivedAt, 'compact') }}</span>
              </span>
            </div>
            <div class="mail-list__row2" :class="{ 'font-weight-medium': item.thread.unreadCount > 0 }">{{ item.thread.message.subject }}</div>
            <div class="mail-list__row3 text-medium-emphasis">{{ item.thread.message.preview }}</div>
          </div>
          <template #append>
            <v-progress-circular
              v-if="item.thread.message.pendingRead"
              indeterminate
              size="14"
              width="2"
              color="primary"
            />
            <v-chip v-else-if="item.thread.unreadCount > 0" size="x-small" color="primary" variant="tonal">
              {{ item.thread.unreadCount }}
            </v-chip>
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
      <v-list-item
        v-if="!props.isPop3"
        prepend-icon="mdi-label-outline"
        :title="t('labels.manageTitle')"
        @click="$emit('context-manage-labels', ctxMenu.target.value!.id)"
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
import EmptyState from '@/components/ui/EmptyState.vue'
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
  batchBusy?: boolean
  selectedIds: Set<string>
  title: string
  folders?: MailboxFolder[]
  currentFolderId?: string | null
  currentFolderKind?: string | null
}>()

const emit = defineEmits<{
  'select-message': [payload: { messageId: string; shiftKey: boolean; accelKey: boolean }]
  'toggle-star': [messageId: string]
  'toggle-selection': [payload: { messageId: string; shiftKey: boolean; accelKey: boolean }]
  'select-all': []
  'clear-selection': []
  'mark-all-read': []
  'batch-delete': []
  'batch-archive': []
  'batch-mark-read': []
  'batch-mark-unread': []
  'batch-move': [folderId: string]
  'batch-manage-labels': []
  'context-reply': [messageId: string]
  'context-reply-all': [messageId: string]
  'context-forward': [messageId: string]
  'context-toggle-read': [messageId: string]
  'context-manage-labels': [messageId: string]
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
const batchBusy = computed(() => Boolean(props.batchBusy))

const onToggleSelectAll = (val: boolean | null) => {
  if (val) {
    emit('select-all')
  } else {
    emit('clear-selection')
  }
}

const onItemPointerDown = (event: MouseEvent) => {
  if (event.shiftKey || event.ctrlKey || event.metaKey) {
    event.preventDefault()
  }
}

const onSelectMessage = (messageId: string, event: MouseEvent | KeyboardEvent) => {
  emit('select-message', {
    messageId,
    shiftKey: 'shiftKey' in event ? event.shiftKey : false,
    accelKey: 'metaKey' in event ? event.metaKey || event.ctrlKey : false,
  })
}

const onToggleSelection = (messageId: string, event: MouseEvent | KeyboardEvent) => {
  emit('toggle-selection', {
    messageId,
    shiftKey: 'shiftKey' in event ? event.shiftKey : false,
    accelKey: 'metaKey' in event ? event.metaKey || event.ctrlKey : false,
  })
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

const formatDate = (value: string, mode: 'full' | 'compact' = 'full') => {
  const date = new Date(value)
  const full = new Intl.DateTimeFormat(locale.value, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(date)

  if (mode === 'full') {
    return full
  }

  const compact = new Intl.DateTimeFormat(locale.value, {
    month: 'numeric',
    day: 'numeric',
  }).format(date)

  return compact
}
</script>

<style scoped>
.mail-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  padding: 12px 12px 0;
  container-type: inline-size;
}

.mail-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.mail-list__header-primary,
.mail-list__header-meta {
  display: flex;
  align-items: center;
  min-width: 0;
}

.mail-list__header-primary {
  gap: 10px;
  flex: 1;
}

.mail-list__header-meta {
  gap: 8px;
  flex-shrink: 0;
  justify-content: flex-end;
}

.mail-list__title {
  min-width: 0;
  font-weight: 600;
  letter-spacing: 0.01em;
}

.mail-list__total {
  white-space: nowrap;
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
  display: grid;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 18px;
  background: rgba(var(--v-theme-primary), 0.08);
  border: 1px solid rgba(var(--v-theme-primary), 0.08);
}

.mail-list__batch-toolbar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.mail-list__batch-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.mail-list__batch-actions :deep(.v-btn) {
  min-width: 0;
  width: 38px;
  height: 38px;
}

.mail-list__batch-cancel {
  flex-shrink: 0;
  margin-right: -6px;
}

.mail-list__batch-toolbar--busy {
  opacity: 0.72;
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
  margin-bottom: 3px;
  padding-block: 4px;
  border-radius: 16px;
}

.mail-list__item :deep(.v-list-item__prepend) {
  align-self: center;
  margin-inline-end: 12px;
}

.mail-list__item :deep(.v-list-item__content) {
  min-width: 0;
  padding: 6px 0;
}

.mail-list__item :deep(.v-list-item__append) {
  align-self: center;
  padding-left: 6px;
}

.mail-list__lead {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  width: 28px;
  min-width: 28px;
}

.mail-list__lead-icon,
.mail-list__lead-spacer {
  width: 18px;
  height: 18px;
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

.mail-list__item--pending {
  opacity: 0.72;
}

.mail-list__row1 {
  display: flex;
  align-items: center;
  gap: 8px;
  line-height: 1.35;
  margin-bottom: 3px;
}

.mail-list__content {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.mail-list__from-group {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  width: 100%;
}

.mail-list__from {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.875rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-list__date {
  display: inline-block;
  flex: none;
  margin-left: auto;
  white-space: nowrap;
  text-align: right;
  font-size: 0.75rem;
  line-height: 1.25;
}

.mail-list__date-compact { display: none; }

.mail-list__star {
  flex: none;
  cursor: pointer;
}

.mail-list__star--pending {
  opacity: 0.45;
}

.mail-list__row2 {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 0.875rem;
  line-height: 1.35;
  color: rgba(var(--v-theme-on-surface), 0.92);
}

.mail-list__row3 {
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  font-size: 0.8125rem;
  line-height: 1.3;
  word-break: break-word;
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
    gap: 10px;
  }

  .mail-list__header-primary,
  .mail-list__header-meta {
    width: 100%;
  }

  .mail-list__header-meta {
    justify-content: flex-start;
    flex-wrap: wrap;
    padding-left: 2px;
  }

  .mail-list__batch-toolbar {
    padding: 12px;
  }

  .mail-list__batch-toolbar-header {
    align-items: flex-start;
  }

  .mail-list__date {
    max-width: 112px;
    white-space: nowrap;
    line-height: 1.3;
  }

}

@container (max-width: 440px) {
  .mail-list__date-full {
    display: none;
  }

  .mail-list__date-compact {
    display: inline;
  }
}
</style>
