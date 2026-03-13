<template>
  <div class="mail-sidebar">
    <v-card class="mail-sidebar__panel" color="surface">
      <div class="mail-sidebar__header d-flex align-center justify-space-between ga-3 flex-wrap">
        <div>
          <div class="text-overline">{{ t('sidebar.accounts') }}</div>
          <div class="text-h6">{{ t('sidebar.unifiedInbox') }}</div>
        </div>
        <v-btn icon="mdi-plus" size="small" @click="$emit('add-account')" />
      </div>

      <v-list v-if="accounts.length" class="mail-sidebar__accounts" lines="two">
        <v-list-item
          v-for="account in accounts"
          :key="account.id"
          :active="account.id === currentAccountId"
          rounded="xl"
          @click="$emit('select-account', account.id)"
          @contextmenu="accountCtx.open($event, account)"
        >
          <template #prepend>
            <v-avatar :color="account.color" size="36">{{ account.initials }}</v-avatar>
          </template>
          <v-list-item-title>{{ account.name }}</v-list-item-title>
          <v-list-item-subtitle>{{ account.email }}</v-list-item-subtitle>
          <template #append>
            <v-chip size="small" :color="account.status === 'attention' ? 'error' : 'primary'">
              {{ account.unreadCount }}
            </v-chip>
            <v-btn
              class="ml-1"
              icon="mdi-delete-outline"
              size="x-small"
              variant="text"
              @click.stop="confirmDelete(account)"
            />
          </template>
        </v-list-item>
      </v-list>

      <div v-else class="mail-sidebar__empty">
        <v-icon icon="mdi-email-plus-outline" size="36" class="mb-2" />
        <div class="text-body-2 text-medium-emphasis mb-3">{{ t('sidebar.noAccounts') }}</div>
        <v-btn size="small" prepend-icon="mdi-plus" @click="$emit('add-account')">{{ t('sidebar.addAccount') }}</v-btn>
      </div>
    </v-card>

    <v-card class="mail-sidebar__panel" color="surface">
      <div class="mail-sidebar__header d-flex align-center justify-space-between ga-3 flex-wrap">
        <div>
          <div class="text-overline">{{ t('sidebar.folders') }}</div>
          <div class="text-h6">{{ currentAccount?.name ?? t('sidebar.selectAccount') }}</div>
        </div>
        <v-btn prepend-icon="mdi-pencil-plus-outline" @click="$emit('compose')">{{ t('sidebar.compose') }}</v-btn>
      </div>

      <v-progress-linear v-if="isFoldersLoading" indeterminate color="primary" class="mb-2" />

      <v-list v-if="folders.length" nav>
        <v-list-item
          v-for="folder in folders"
          :key="folder.id"
          :active="folder.id === currentFolderId"
          rounded="xl"
          @click="$emit('select-folder', folder.id)"
          @contextmenu="folderCtx.open($event, folder)"
        >
          <template #prepend>
            <v-icon :icon="folder.icon" />
          </template>
          <v-list-item-title>{{ folderDisplayName(folder) }}</v-list-item-title>
          <template #append>
            <div class="mail-sidebar__folder-counts d-flex align-center ga-1">
              <v-chip
                v-if="folder.unreadCount > 0"
                size="x-small"
                color="primary"
                variant="flat"
                density="comfortable"
              >
                {{ folder.unreadCount }}
              </v-chip>
              <span class="text-caption text-medium-emphasis">{{ folder.totalCount }}</span>
            </div>
          </template>
        </v-list-item>
      </v-list>

      <div v-else class="mail-sidebar__empty">
        <v-icon icon="mdi-folder-outline" size="36" class="mb-2" />
        <div class="text-body-2 text-medium-emphasis">
          {{ accounts.length ? t('sidebar.selectAccountToView') : t('sidebar.addAccountToStart') }}
        </div>
      </div>
    </v-card>

    <!-- Account context menu -->
    <ContextMenu v-model="accountCtx.isOpen.value" :x="accountCtx.x.value" :y="accountCtx.y.value">
      <v-list-item prepend-icon="mdi-sync" :title="t('shell.sync')" @click="$emit('sync-account', accountCtx.target.value!.id)" />
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
import { ref } from 'vue'
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

defineProps<{
  accounts: MailAccount[]
  currentAccount: MailAccount | null
  currentAccountId: string | null
  currentFolderId: string | null
  folders: MailboxFolder[]
  isFoldersLoading?: boolean
}>()

const emit = defineEmits<{
  'select-account': [accountId: string]
  'select-folder': [folderId: string]
  'delete-account': [accountId: string]
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
  display: grid;
  gap: 16px;
  padding: 16px;
}

.mail-sidebar__panel {
  padding: 16px;
}

.mail-sidebar__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.mail-sidebar__header :deep(.v-btn) {
  flex-shrink: 0;
}

.mail-sidebar__accounts {
  padding-inline: 0;
}

.mail-sidebar__empty {
  display: grid;
  place-items: center;
  text-align: center;
  padding: 24px 16px;
}

@media (max-width: 840px) {
  .mail-sidebar {
    padding: 16px 16px 12px;
  }
}

@media (max-width: 600px) {
  .mail-sidebar {
    gap: 12px;
    padding: 12px;
  }

  .mail-sidebar__panel {
    padding: 12px;
  }

  .mail-sidebar__header {
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .mail-sidebar__header :deep(.v-btn) {
    width: 100%;
    justify-content: center;
  }
}
</style>
