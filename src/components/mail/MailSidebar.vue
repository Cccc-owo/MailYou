<template>
  <div class="mail-sidebar">
    <div class="mail-sidebar__section">
      <div class="mail-sidebar__section-header">
        <v-icon icon="mdi-account-multiple-outline" size="18" />
        <span>{{ t('sidebar.accounts') }}</span>
        <v-spacer />
        <v-tooltip :text="t('sidebar.addAccount')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-plus" size="x-small" variant="text" @click="$emit('add-account')" />
          </template>
        </v-tooltip>
      </div>

      <v-list v-if="accounts.length" class="mail-sidebar__list" lines="two">
        <v-list-item
          v-for="account in accounts"
          :key="account.id"
          :active="account.id === currentAccountId"
          @click="$emit('select-account', account.id)"
          @contextmenu="accountCtx.open($event, account)"
        >
          <template #prepend>
            <v-avatar :color="account.color" size="32">
              <span class="text-caption">{{ account.initials }}</span>
            </v-avatar>
          </template>
          <v-list-item-title class="text-body-2">{{ account.name }}</v-list-item-title>
          <v-list-item-subtitle class="text-caption">{{ account.email }}</v-list-item-subtitle>
          <template #append>
            <v-chip size="x-small" :color="account.status === 'attention' ? 'error' : 'primary'" variant="tonal">
              {{ account.unreadCount }}
            </v-chip>
          </template>
        </v-list-item>
      </v-list>

      <div v-else class="mail-sidebar__empty">
        <v-icon icon="mdi-email-plus-outline" size="32" class="mb-2" />
        <div class="text-body-2 text-medium-emphasis mb-3">{{ t('sidebar.noAccounts') }}</div>
        <v-btn size="small" prepend-icon="mdi-plus" @click="$emit('add-account')">{{ t('sidebar.addAccount') }}</v-btn>
      </div>
    </div>

    <v-divider />

    <div class="mail-sidebar__section">
      <div class="mail-sidebar__section-header">
        <v-icon icon="mdi-folder-outline" size="18" />
        <span>{{ currentAccount?.name ?? t('sidebar.selectAccount') }}</span>
        <v-spacer />
        <v-btn size="small" variant="tonal" prepend-icon="mdi-pencil-plus-outline" @click="$emit('compose')">{{ t('sidebar.compose') }}</v-btn>
      </div>

      <v-progress-linear v-if="isFoldersLoading" indeterminate color="primary" class="mb-1" />

      <v-list v-if="visibleFolders.length" class="mail-sidebar__list" nav density="compact">
        <v-list-item
          v-for="folder in visibleFolders"
          :key="folder.id"
          :active="folder.id === currentFolderId"
          @click="$emit('select-folder', folder.id)"
          @contextmenu="folderCtx.open($event, folder)"
        >
          <template #prepend>
            <v-icon :icon="folder.icon" size="20" />
          </template>
          <v-list-item-title class="text-body-2">{{ folderDisplayName(folder) }}</v-list-item-title>
          <template #append>
            <div class="mail-sidebar__folder-counts d-flex align-center ga-1">
              <v-chip
                v-if="folder.unreadCount > 0"
                size="x-small"
                color="primary"
                variant="tonal"
              >
                {{ folder.unreadCount }}
              </v-chip>
              <span class="text-caption text-medium-emphasis">{{ folder.totalCount }}</span>
            </div>
          </template>
        </v-list-item>
      </v-list>

      <v-alert
        v-if="currentAccount?.incomingProtocol === 'pop3' && visibleFolders.length > 0"
        type="info"
        variant="tonal"
        density="compact"
        class="ma-2 text-caption"
      >
        {{ t('sidebar.pop3LimitedFeatures') }}
      </v-alert>

      <div v-else class="mail-sidebar__empty">
        <v-icon icon="mdi-folder-outline" size="32" class="mb-2" />
        <div class="text-body-2 text-medium-emphasis">
          {{ accounts.length ? t('sidebar.selectAccountToView') : t('sidebar.addAccountToStart') }}
        </div>
      </div>
    </div>

    <!-- Account context menu -->
    <ContextMenu v-model="accountCtx.isOpen.value" :x="accountCtx.x.value" :y="accountCtx.y.value">
      <v-list-item prepend-icon="mdi-sync" :title="t('shell.sync')" @click="$emit('sync-account', accountCtx.target.value!.id)" />
      <v-list-item prepend-icon="mdi-pencil-outline" :title="t('sidebar.editAccount')" @click="$emit('edit-account', accountCtx.target.value!.id)" />
      <v-divider />
      <v-list-item prepend-icon="mdi-delete-outline" :title="t('sidebar.deleteAccount')" base-color="error" @click="confirmDelete(accountCtx.target.value!)" />
    </ContextMenu>

    <!-- Folder context menu -->
    <ContextMenu v-model="folderCtx.isOpen.value" :x="folderCtx.x.value" :y="folderCtx.y.value">
      <v-list-item prepend-icon="mdi-email-check-outline" :title="t('mailList.markAllRead')" @click="$emit('mark-folder-read', folderCtx.target.value!.id)" />
    </ContextMenu>
  </div>

  <v-dialog v-model="deleteDialog" max-width="400" persistent>
    <v-card>
      <v-card-title>{{ t('sidebar.deleteAccount') }}</v-card-title>
      <v-card-text>
        {{ t('sidebar.deleteConfirmText', { name: pendingDeleteAccount?.name, email: pendingDeleteAccount?.email }) }}
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn @click="deleteDialog = false">{{ t('common.cancel') }}</v-btn>
        <v-btn color="error" variant="flat" @click="emitDelete">{{ t('common.delete') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MailAccount } from '@/types/account'
import type { MailboxFolder } from '@/types/mail'
import ContextMenu from '@/components/ContextMenu.vue'
import { useContextMenu } from '@/composables/useContextMenu'

const { t } = useI18n()
const accountCtx = useContextMenu<MailAccount>()
const folderCtx = useContextMenu<MailboxFolder>()

const folderDisplayName = (folder: MailboxFolder) =>
  folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name

const props = defineProps<{
  accounts: MailAccount[]
  currentAccount: MailAccount | null
  currentAccountId: string | null
  currentFolderId: string | null
  folders: MailboxFolder[]
  isFoldersLoading?: boolean
}>()

// Filter folders for POP3: only show inbox and starred
const visibleFolders = computed(() => {
  if (props.currentAccount?.incomingProtocol === 'pop3') {
    return props.folders.filter(f => f.kind === 'inbox' || f.kind === 'starred')
  }
  return props.folders
})

const emit = defineEmits<{
  'select-account': [accountId: string]
  'select-folder': [folderId: string]
  'delete-account': [accountId: string]
  'edit-account': [accountId: string]
  compose: []
  'add-account': []
  'sync-account': [accountId: string]
  'mark-folder-read': [folderId: string]
}>()

const deleteDialog = ref(false)
const pendingDeleteAccount = ref<MailAccount | null>(null)

const confirmDelete = (account: MailAccount) => {
  pendingDeleteAccount.value = account
  deleteDialog.value = true
}

const emitDelete = () => {
  if (pendingDeleteAccount.value) {
    emit('delete-account', pendingDeleteAccount.value.id)
  }
  deleteDialog.value = false
  pendingDeleteAccount.value = null
}
</script>

<style scoped>
.mail-sidebar {
  display: flex;
  flex-direction: column;
  padding: 8px 0;
}

.mail-sidebar__section {
  padding: 0 8px;
}

.mail-sidebar__section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 8px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: rgb(var(--v-theme-primary));
}

.mail-sidebar__list {
  padding: 0;
}

.mail-sidebar__empty {
  display: grid;
  place-items: center;
  text-align: center;
  padding: 24px 16px;
}

@media (max-width: 600px) {
  .mail-sidebar__section-header :deep(.v-btn) {
    width: 100%;
    justify-content: center;
  }
}
</style>
