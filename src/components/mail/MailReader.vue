<template>
  <div v-if="message" class="mail-reader" @contextmenu="openReaderMenu($event)">
    <div class="mail-reader__toolbar">
      <div class="mail-reader__toolbar-subject text-subtitle-1">{{ message.subject }}</div>
      <div class="mail-reader__toolbar-actions">
        <v-tooltip :text="t('reader.reply')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-reply-outline" variant="text" size="small" @click="$emit('reply')" />
          </template>
        </v-tooltip>
        <v-tooltip :text="t('reader.replyAll')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-reply-all-outline" variant="text" size="small" @click="$emit('reply-all')" />
          </template>
        </v-tooltip>
        <v-tooltip :text="t('reader.forward')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-arrow-top-right" variant="text" size="small" @click="$emit('forward')" />
          </template>
        </v-tooltip>
        <v-tooltip :text="message.isRead ? t('reader.markUnread') : t('reader.markRead')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" :icon="message.isRead ? 'mdi-email-outline' : 'mdi-email-open-outline'" variant="text" size="small" @click="$emit('toggle-read')" />
          </template>
        </v-tooltip>

        <v-menu v-if="moveTargetFolders.length > 0">
          <template #activator="{ props: menuProps }">
            <v-tooltip :text="t('reader.moveTo')" location="bottom">
              <template #activator="{ props: tip }">
                <v-btn v-bind="{ ...tip, ...menuProps }" icon="mdi-folder-move-outline" variant="text" size="small" />
              </template>
            </v-tooltip>
          </template>
          <v-list density="compact">
            <v-list-item
              v-for="folder in moveTargetFolders"
              :key="folder.id"
              :prepend-icon="folder.icon"
              :title="folderDisplayName(folder)"
              @click="$emit('move', folder.id)"
            />
          </v-list>
        </v-menu>

        <template v-if="isTrashOrArchive">
          <v-tooltip :text="t('reader.restoreToInbox')" location="bottom">
            <template #activator="{ props: tip }">
              <v-btn v-bind="tip" icon="mdi-inbox-arrow-down" variant="text" size="small" @click="$emit('restore')" />
            </template>
          </v-tooltip>
        </template>
        <template v-else>
          <v-tooltip :text="t('reader.archive')" location="bottom">
            <template #activator="{ props: tip }">
              <v-btn v-bind="tip" icon="mdi-archive-outline" variant="text" size="small" @click="$emit('archive')" />
            </template>
          </v-tooltip>
        </template>

        <v-tooltip :text="t('common.delete')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-delete-outline" variant="text" size="small" color="error" @click="$emit('delete')" />
          </template>
        </v-tooltip>
      </div>
    </div>

    <div class="mail-reader__scroll">
    <div class="mail-reader__message">
      <div class="mail-reader__meta">
        <div>
          <div class="d-flex align-center ga-1">
            <div class="text-subtitle-1 font-weight-medium">{{ message.from }}</div>
            <v-btn
              icon="mdi-account-plus-outline"
              variant="text"
              size="x-small"
              :title="t('contacts.saveToContacts')"
              @click="$emit('save-contact', { name: message.from, email: message.fromEmail })"
            />
          </div>
          <div class="text-body-2 text-medium-emphasis">{{ message.fromEmail }}</div>
        </div>
        <div class="text-body-2 text-medium-emphasis text-right">
          <div>{{ formattedDate }}</div>
          <div>{{ t('reader.to', { recipients: message.to.join(', ') }) }}</div>
          <div v-if="message.cc.length > 0">{{ t('reader.cc', { recipients: message.cc.join(', ') }) }}</div>
        </div>
      </div>

      <div class="mail-reader__chips d-flex flex-wrap ga-2">
        <v-chip v-for="label in message.labels" :key="label" size="small" color="secondary">{{ label }}</v-chip>
        <v-chip v-if="message.hasAttachments" size="small" color="primary">{{ t('reader.attachmentsCount', { count: message.attachments.length }) }}</v-chip>
      </div>

      <div class="mail-reader__body text-body-1" v-html="sanitizedBody" @click="handleBodyClick" />

      <v-list v-if="message.attachments.length" class="mail-reader__attachments">
        <v-list-item v-for="attachment in message.attachments" :key="attachment.id" rounded="xl">
          <template #prepend>
            <v-icon icon="mdi-paperclip" />
          </template>
          <v-list-item-title>{{ attachment.fileName }}</v-list-item-title>
          <v-list-item-subtitle>{{ formatSize(attachment.sizeBytes) }}</v-list-item-subtitle>
          <template #append>
            <v-tooltip :text="t('reader.download')" location="bottom">
              <template #activator="{ props: tip }">
                <v-btn
                  v-bind="tip"
                  icon="mdi-download"
                  variant="text"
                  size="small"
                  :loading="downloadingId === attachment.id"
                  @click="downloadAttachment(attachment)"
                />
              </template>
            </v-tooltip>
          </template>
        </v-list-item>
      </v-list>
    </div>
    </div>

    <!-- Right-click context menu -->
    <ContextMenu v-model="ctxMenu.isOpen.value" :x="ctxMenu.x.value" :y="ctxMenu.y.value">
      <v-list-item v-if="hasSelection" prepend-icon="mdi-content-copy" :title="t('reader.copy')" @click="copySelection" />
      <template v-if="targetHref">
        <v-list-item prepend-icon="mdi-open-in-new" :title="t('reader.openLinkInBrowser')" @click="openLinkInBrowser" />
        <v-list-item prepend-icon="mdi-link-variant" :title="t('reader.copyLinkAddress')" @click="copyLinkAddress" />
      </template>
      <template v-if="targetImgSrc">
        <v-list-item prepend-icon="mdi-image-outline" :title="t('reader.copyImage')" @click="copyImage" />
        <v-list-item prepend-icon="mdi-image-multiple-outline" :title="t('reader.copyImageAddress')" @click="copyImageAddress" />
      </template>
      <v-divider v-if="hasSelection || targetHref || targetImgSrc" />
      <v-list-item prepend-icon="mdi-select-all" :title="t('reader.selectAll')" @click="selectAllBody" />
    </ContextMenu>
  </div>

  <div v-else class="mail-reader__empty">
    <v-icon :icon="emptyStateIcon" size="48" class="mb-4" />
    <div class="text-h5 mb-2">{{ emptyStateTitle }}</div>
    <div class="text-body-1 text-medium-emphasis">{{ emptyStateDescription }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import DOMPurify from 'dompurify'
import type { AttachmentMeta, MailMessage, MailboxFolder } from '@/types/mail'
import ContextMenu from '@/components/ContextMenu.vue'
import { useContextMenu } from '@/composables/useContextMenu'
import { mailRepository } from '@/services/mail'

const { t, locale } = useI18n()
const ctxMenu = useContextMenu()
const hasSelection = ref(false)
const targetHref = ref<string | null>(null)
const targetImgSrc = ref<string | null>(null)
const downloadingId = ref<string | null>(null)

const downloadAttachment = async (attachment: AttachmentMeta) => {
  if (!props.message) return
  downloadingId.value = attachment.id
  try {
    const content = await mailRepository.getAttachmentContent(
      props.message.accountId,
      props.message.id,
      attachment.id,
    )
    const binary = atob(content.dataBase64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i)
    }
    const blob = new Blob([bytes], { type: content.mimeType })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = content.fileName
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  } catch (err) {
    console.error('Failed to download attachment:', err)
  } finally {
    downloadingId.value = null
  }
}

const findAncestor = (el: HTMLElement | null, tag: string): HTMLElement | null => {
  while (el) {
    if (el.tagName === tag) return el
    el = el.parentElement
  }
  return null
}

const openReaderMenu = (event: MouseEvent) => {
  const target = event.target as HTMLElement

  const sel = window.getSelection()
  hasSelection.value = Boolean(sel && sel.toString().trim().length > 0)

  const anchor = findAncestor(target, 'A') as HTMLAnchorElement | null
  targetHref.value = anchor?.href ?? null

  const img = findAncestor(target, 'IMG') as HTMLImageElement | null
  targetImgSrc.value = img?.src ?? null

  ctxMenu.open(event)
}

const copySelection = () => {
  const sel = window.getSelection()
  if (sel) {
    navigator.clipboard.writeText(sel.toString())
  }
}

const copyLinkAddress = () => {
  if (targetHref.value) {
    navigator.clipboard.writeText(targetHref.value)
  }
}

const openUrlExternal = (url: string) => {
  const wc = (window as unknown as Record<string, unknown>).windowControls as
    | { openExternal?: (url: string) => Promise<void> }
    | undefined
  if (wc?.openExternal) {
    wc.openExternal(url)
  } else {
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}

const openLinkInBrowser = () => {
  if (targetHref.value) openUrlExternal(targetHref.value)
}

const handleBodyClick = (event: MouseEvent) => {
  const anchor = findAncestor(event.target as HTMLElement, 'A') as HTMLAnchorElement | null
  if (anchor?.href) {
    event.preventDefault()
    openUrlExternal(anchor.href)
  }
}

const copyImage = async () => {
  if (!targetImgSrc.value) return
  try {
    const res = await fetch(targetImgSrc.value)
    const blob = await res.blob()
    await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })])
  } catch {
    // fallback: copy the URL instead
    navigator.clipboard.writeText(targetImgSrc.value)
  }
}

const copyImageAddress = () => {
  if (targetImgSrc.value) {
    navigator.clipboard.writeText(targetImgSrc.value)
  }
}

const selectAllBody = () => {
  const el = document.querySelector('.mail-reader__body')
  if (!el) return
  const range = document.createRange()
  range.selectNodeContents(el)
  const sel = window.getSelection()
  if (sel) {
    sel.removeAllRanges()
    sel.addRange(range)
  }
}

const folderDisplayName = (folder: MailboxFolder) =>
  folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name

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
  'save-contact': [data: { name: string; email: string }]
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

  return new Intl.DateTimeFormat(locale.value, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(props.message.receivedAt))
})

const emptyStateTitle = computed(() => {
  if (props.hasSearchQuery && !props.hasMessages) {
    return t('reader.noMatchingMessage')
  }

  if (!props.hasMessages) {
    return t('reader.noMessageSelected')
  }

  return t('reader.selectMessage')
})

const emptyStateDescription = computed(() => {
  if (props.hasSearchQuery && !props.hasMessages) {
    return t('reader.noMatchingMessageHint')
  }

  if (!props.hasMessages) {
    return t('reader.noMessageSelectedHint')
  }

  return t('reader.selectMessageHint')
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
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.mail-reader__toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  padding: 10px 16px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}

.mail-reader__toolbar-subject {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}

.mail-reader__toolbar-actions {
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  gap: 4px;
  flex-shrink: 0;
}

.mail-reader__scroll {
  flex: 1;
  overflow: auto;
  padding: 16px;
}

.mail-reader__message {
  padding: 0;
}

.mail-reader__meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 12px;
  margin-bottom: 16px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}

.mail-reader__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
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
</style>
