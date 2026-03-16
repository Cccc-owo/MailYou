<template>
  <div v-if="message" class="mail-reader" @contextmenu="openReaderMenu($event)">
    <!-- Toolbar: primary actions with labels -->
    <div class="mail-reader__toolbar">
      <div class="mail-reader__toolbar-primary">
        <v-btn variant="text" size="small" prepend-icon="mdi-reply-outline" @click="$emit('reply')">{{ t('reader.reply') }}</v-btn>
        <v-btn variant="text" size="small" prepend-icon="mdi-reply-all-outline" @click="$emit('reply-all')">{{ t('reader.replyAll') }}</v-btn>
        <v-btn variant="text" size="small" prepend-icon="mdi-share-outline" @click="$emit('forward')">{{ t('reader.forward') }}</v-btn>
        <v-btn variant="text" size="small" prepend-icon="mdi-delete-outline" color="error" @click="$emit('delete')">{{ t('common.delete') }}</v-btn>
      </div>
      <div class="mail-reader__toolbar-secondary">
        <v-tooltip :text="t('reader.exportPdf')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" variant="text" size="small" icon="mdi-file-pdf-box" @click="$emit('export-pdf')" />
          </template>
        </v-tooltip>
        <v-menu>
          <template #activator="{ props: menuProps }">
            <v-btn v-bind="menuProps" variant="text" size="small" prepend-icon="mdi-dots-horizontal-circle-outline">
              {{ t('reader.more') }}
            </v-btn>
          </template>
          <v-list density="compact">
            <v-list-item
              :prepend-icon="message.isRead ? 'mdi-email-outline' : 'mdi-email-open-outline'"
              :title="message.isRead ? t('reader.markUnread') : t('reader.markRead')"
              @click="$emit('toggle-read')"
            />
            <v-list-item
              v-if="isTrashArchiveOrJunk"
              prepend-icon="mdi-inbox-arrow-down"
              :title="currentFolderKind === 'junk' ? t('reader.notSpam') : t('reader.restoreToInbox')"
              @click="$emit('restore')"
            />
            <v-list-item
              v-else-if="!isPop3"
              prepend-icon="mdi-archive-outline"
              :title="t('reader.archive')"
              @click="$emit('archive')"
            />
            <v-list-item
              v-if="!isTrashArchiveOrJunk && !isPop3"
              prepend-icon="mdi-alert-circle-outline"
              :title="t('reader.markSpam')"
              @click="$emit('mark-spam')"
            />
            <v-divider v-if="moveTargetFolders.length > 0" />
            <v-list-item
              v-for="folder in moveTargetFolders"
              :key="folder.id"
              :prepend-icon="folder.icon"
              :title="t('reader.moveTo') + ' ' + folderDisplayName(folder)"
              @click="$emit('move', folder.id)"
            />
          </v-list>
        </v-menu>
      </div>
    </div>

    <div class="mail-reader__scroll">
    <div v-if="threadMessages.length > 1" class="mail-reader__conversation">
      <div class="mail-reader__conversation-header">
        <span class="text-subtitle-2">{{ t('reader.conversation') }}</span>
        <v-chip size="x-small" variant="tonal" color="secondary">
          {{ t('reader.messagesInConversation', { count: threadMessages.length }) }}
        </v-chip>
      </div>
      <v-list density="compact" class="mail-reader__conversation-list">
        <v-list-item
          v-for="threadMessage in threadMessages"
          :key="threadMessage.id"
          rounded="lg"
          :active="threadMessage.id === message.id"
          @click="$emit('select-thread-message', threadMessage.id)"
        >
          <v-list-item-title>{{ threadMessage.from }}</v-list-item-title>
          <v-list-item-subtitle>
            {{ formatConversationDate(threadMessage.receivedAt) }}
          </v-list-item-subtitle>
          <template #append>
            <div class="d-flex align-center ga-2">
              <v-icon v-if="threadMessage.hasAttachments" icon="mdi-paperclip" size="16" class="text-medium-emphasis" />
              <v-icon v-if="!threadMessage.isRead" icon="mdi-circle" size="10" color="primary" />
            </div>
          </template>
        </v-list-item>
      </v-list>
    </div>
    <div class="mail-reader__message">
      <!-- Subject + Star -->
      <div class="mail-reader__subject-wrap">
        <h2 ref="subjectEl" class="mail-reader__subject" :class="{ 'mail-reader__subject--collapsed': subjectCollapsed }">
          {{ message.subject }}
        </h2>
        <v-btn
          v-if="subjectOverflows"
          :icon="subjectCollapsed ? 'mdi-chevron-down' : 'mdi-chevron-up'"
          variant="text"
          size="x-small"
          density="compact"
          class="mail-reader__subject-toggle"
          @click="subjectCollapsed = !subjectCollapsed"
        />
        <v-btn
          :icon="message.isStarred ? 'mdi-star' : 'mdi-star-outline'"
          :color="message.isStarred ? 'warning' : undefined"
          variant="text"
          size="small"
          density="compact"
          class="ml-1 flex-shrink-0"
          @click="$emit('toggle-star')"
        />
      </div>

      <!-- Sender + Recipients -->
      <div class="mail-reader__meta">
        <v-avatar color="primary" size="36" class="mail-reader__avatar flex-shrink-0">
          <v-img v-if="senderAvatarUrl" :src="senderAvatarUrl" cover />
          <span v-else class="text-body-2 font-weight-medium">{{ senderInitials }}</span>
        </v-avatar>
        <div class="mail-reader__meta-content">
          <div class="mail-reader__meta-top">
            <div class="mail-reader__sender-line">
              <EmailContactPopover
                :name="message.from"
                :email="message.fromEmail"
                @compose="(d) => $emit('compose-to', d)"
                @save-contact="(d) => $emit('save-contact', d)"
                @view-contact="(c) => $emit('view-contact', c)"
              >
                <span class="font-weight-medium">{{ message.from }}</span>
              </EmailContactPopover>
              <EmailContactPopover
                :name="message.from"
                :email="message.fromEmail"
                @compose="(d) => $emit('compose-to', d)"
                @save-contact="(d) => $emit('save-contact', d)"
                @view-contact="(c) => $emit('view-contact', c)"
              >
                <span class="text-medium-emphasis">&lt;{{ message.fromEmail }}&gt;</span>
              </EmailContactPopover>
            </div>
            <span class="mail-reader__date text-medium-emphasis">{{ formattedDate }}</span>
          </div>
          <div class="mail-reader__recipients text-medium-emphasis">
            <span>
              {{ t('reader.to') }}
              <template v-for="(addr, i) in message.to" :key="'to-'+i">
                <span v-if="i > 0">, </span>
                <EmailContactPopover
                  :name="parseAddr(addr).name"
                  :email="parseAddr(addr).email"
                  @compose="(d) => $emit('compose-to', d)"
                  @save-contact="(d) => $emit('save-contact', d)"
                  @view-contact="(c) => $emit('view-contact', c)"
                >{{ addr }}</EmailContactPopover>
              </template>
            </span>
            <span v-if="message.cc.length > 0" class="ml-2">
              {{ t('reader.ccLabel') }}
              <template v-for="(addr, i) in message.cc" :key="'cc-'+i">
                <span v-if="i > 0">, </span>
                <EmailContactPopover
                  :name="parseAddr(addr).name"
                  :email="parseAddr(addr).email"
                  @compose="(d) => $emit('compose-to', d)"
                  @save-contact="(d) => $emit('save-contact', d)"
                  @view-contact="(c) => $emit('view-contact', c)"
                >{{ addr }}</EmailContactPopover>
              </template>
            </span>
          </div>
        </div>
      </div>

      <div v-if="message.labels.length || message.hasAttachments" class="mail-reader__chips d-flex flex-wrap ga-2">
        <v-chip v-for="label in message.labels" :key="label" size="small" color="secondary">{{ label }}</v-chip>
        <v-chip v-if="message.hasAttachments" size="small" color="primary">{{ t('reader.attachmentsCount', { count: message.attachments.length }) }}</v-chip>
      </div>

      <v-alert
        v-if="hasBlockedImages"
        type="info"
        variant="tonal"
        density="compact"
        class="mb-3"
      >
        <template #text>
          <span>{{ t('reader.imagesBlocked') }}</span>
          <v-btn variant="text" size="small" class="ml-2" @click="allowImagesForMessage = true">{{ t('reader.loadImages') }}</v-btn>
        </template>
      </v-alert>

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
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import DOMPurify from 'dompurify'
import type { AttachmentMeta, MailMessage, MailboxFolder } from '@/types/mail'
import ContextMenu from '@/components/ContextMenu.vue'
import EmailContactPopover from '@/components/mail/EmailContactPopover.vue'
import { useContextMenu } from '@/composables/useContextMenu'
import { mailRepository } from '@/services/mail'
import { useContactsStore } from '@/stores/contacts'
import { useUiStore } from '@/stores/ui'

const { t, locale } = useI18n()
const ctxMenu = useContextMenu()
const contactsStore = useContactsStore()
const uiStore = useUiStore()
const allowImagesForMessage = ref(false)
const hasSelection = ref(false)
const targetHref = ref<string | null>(null)
const targetImgSrc = ref<string | null>(null)
const downloadingId = ref<string | null>(null)

// --- Collapsible subject ---
const subjectEl = ref<HTMLElement | null>(null)
const subjectCollapsed = ref(false)
const subjectOverflows = ref(false)

const checkSubjectOverflow = () => {
  const el = subjectEl.value
  if (!el) { subjectOverflows.value = false; return }
  el.style.webkitLineClamp = 'unset'
  el.style.maxHeight = 'none'
  const full = el.scrollHeight
  el.style.webkitLineClamp = ''
  el.style.maxHeight = ''
  subjectOverflows.value = full > 64
}

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
    threadMessages?: MailMessage[]
    folders?: MailboxFolder[]
    currentFolderId?: string | null
    currentFolderKind?: string | null
    isPop3?: boolean
  }>(),
  {
    hasMessages: false,
    hasSearchQuery: false,
    threadMessages: () => [],
    folders: () => [],
    currentFolderId: null,
    currentFolderKind: null,
    isPop3: false,
  },
)

defineEmits<{
  reply: []
  'reply-all': []
  forward: []
  archive: []
  'mark-spam': []
  restore: []
  delete: []
  'toggle-read': []
  'toggle-star': []
  move: [folderId: string]
  'export-pdf': []
  'save-contact': [data: { name: string; email: string }]
  'compose-to': [data: { name: string; email: string }]
  'view-contact': [contact: import('@/types/contact').Contact]
  'select-thread-message': [messageId: string]
}>()

const isTrashArchiveOrJunk = computed(() =>
  props.currentFolderKind === 'trash' || props.currentFolderKind === 'archive' || props.currentFolderKind === 'junk',
)

const moveTargetFolders = computed(() => {
  if (props.isPop3) return []
  return props.folders.filter(
    (f) => f.id !== props.currentFolderId && f.kind !== 'starred',
  )
})

// --- Subject & sender (depend on props) ---

const senderInitials = computed(() => {
  const name = props.message?.from ?? ''
  return name
    .split(/[\s@]/)
    .filter(Boolean)
    .slice(0, 2)
    .map((s) => s[0].toUpperCase())
    .join('')
})

const senderAvatarUrl = computed(() => {
  if (!props.message) return null
  const matched = contactsStore.contacts.find(
    (c) => c.emails.some((e) => e.toLowerCase() === props.message!.fromEmail.toLowerCase()),
  )
  return contactsStore.avatarUrl(matched)
})

watch(
  () => props.message?.id,
  () => {
    subjectCollapsed.value = false
    allowImagesForMessage.value = false
    nextTick(checkSubjectOverflow)
  },
)

let resizeObserver: ResizeObserver | null = null
onMounted(() => {
  resizeObserver = new ResizeObserver(() => checkSubjectOverflow())
  if (subjectEl.value) resizeObserver.observe(subjectEl.value)
})
onUnmounted(() => resizeObserver?.disconnect())

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

const formatConversationDate = (value: string) =>
  new Intl.DateTimeFormat(locale.value, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(value))

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

  let html = DOMPurify.sanitize(props.message.body, {
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

  const policy = allowImagesForMessage.value ? 'all' : uiStore.imageLoadPolicy
  if (policy === 'noRemote') {
    html = html.replace(/<img\b([^>]*)>/gi, (_match, attrs: string) => {
      const srcMatch = attrs.match(/src\s*=\s*["']([^"']*)["']/i)
      if (srcMatch) {
        const src = srcMatch[1]
        if (src.startsWith('data:') || src.startsWith('mailyou-avatar:')) {
          return `<img${attrs}>`
        }
      }
      return ''
    })
  } else if (policy === 'noHttp') {
    html = html.replace(/<img\b([^>]*)>/gi, (_match, attrs: string) => {
      const srcMatch = attrs.match(/src\s*=\s*["']([^"']*)["']/i)
      if (srcMatch) {
        const src = srcMatch[1]
        if (src.startsWith('http://')) {
          return ''
        }
      }
      return `<img${attrs}>`
    })
  }

  return html
})

const hasBlockedImages = computed(() => {
  if (!props.message || allowImagesForMessage.value || uiStore.imageLoadPolicy === 'all') return false
  const body = props.message.body
  const imgRegex = /<img\b[^>]*src\s*=\s*["']([^"']*)["'][^>]*>/gi
  let m: RegExpExecArray | null
  while ((m = imgRegex.exec(body)) !== null) {
    const src = m[1]
    if (uiStore.imageLoadPolicy === 'noRemote') {
      if (!src.startsWith('data:') && !src.startsWith('cid:') && !src.startsWith('mailyou-avatar:')) {
        return true
      }
    } else if (uiStore.imageLoadPolicy === 'noHttp') {
      if (src.startsWith('http://')) {
        return true
      }
    }
  }
  return false
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

const parseAddr = (addr: string): { name: string; email: string } => {
  const match = addr.match(/^(.+?)\s*<(.+?)>$/)
  if (match) return { name: match[1].trim(), email: match[2].trim() }
  return { name: '', email: addr.trim() }
}

</script>

<style scoped>
.mail-reader {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* Toolbar */
.mail-reader__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  padding: 4px 12px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
  gap: 8px;
}

.mail-reader__toolbar-primary {
  display: flex;
  align-items: center;
  gap: 2px;
}

.mail-reader__toolbar-secondary {
  display: flex;
  align-items: center;
  gap: 2px;
}

.mail-reader__scroll {
  flex: 1;
  overflow: auto;
  padding: 12px 20px;
}

.mail-reader__conversation {
  margin-bottom: 12px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  border-radius: 16px;
  background: rgba(var(--v-theme-on-surface), 0.02);
}

.mail-reader__conversation-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 12px 14px 0;
}

.mail-reader__conversation-list {
  background: transparent;
}

.mail-reader__message {
  padding: 0;
}

/* Subject */
.mail-reader__subject-wrap {
  display: flex;
  align-items: flex-start;
  gap: 2px;
  margin-bottom: 6px;
}

.mail-reader__subject {
  flex: 1;
  min-width: 0;
  font-size: 1.05rem;
  font-weight: 600;
  line-height: 1.4;
  word-break: break-word;
}

.mail-reader__subject--collapsed {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.mail-reader__subject-toggle {
  flex-shrink: 0;
  margin-top: 2px;
}

/* Meta */
.mail-reader__meta {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding-bottom: 10px;
  margin-bottom: 10px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}

.mail-reader__avatar {
  margin-top: 2px;
}

.mail-reader__meta-content {
  flex: 1;
  min-width: 0;
  font-size: 0.8125rem;
}

.mail-reader__meta-top {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 12px;
}

.mail-reader__sender-line {
  display: flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
  flex-wrap: wrap;
}

.mail-reader__date {
  flex-shrink: 0;
  white-space: nowrap;
  font-size: 0.8125rem;
}

.mail-reader__recipients {
  margin-top: 2px;
  line-height: 1.5;
  font-size: 0.8125rem;
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
