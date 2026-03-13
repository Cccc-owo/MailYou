<template>
  <div class="mail-sidebar">
    <v-card class="mail-sidebar__panel" color="surface">
      <div class="mail-sidebar__header d-flex align-center justify-space-between ga-3 flex-wrap">
        <div>
          <div class="text-overline">Accounts</div>
          <div class="text-h6">Unified inbox shell</div>
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
        <div class="text-body-2 text-medium-emphasis mb-3">No accounts configured yet.</div>
        <v-btn size="small" prepend-icon="mdi-plus" @click="$emit('add-account')">Add account</v-btn>
      </div>
    </v-card>

    <v-card class="mail-sidebar__panel" color="surface">
      <div class="mail-sidebar__header d-flex align-center justify-space-between ga-3 flex-wrap">
        <div>
          <div class="text-overline">Folders</div>
          <div class="text-h6">{{ currentAccount?.name ?? 'Select account' }}</div>
        </div>
        <v-btn prepend-icon="mdi-pencil-plus-outline" @click="$emit('compose')">Compose</v-btn>
      </div>

      <v-progress-linear v-if="isFoldersLoading" indeterminate color="primary" class="mb-2" />

      <v-list v-if="folders.length" nav>
        <v-list-item
          v-for="folder in folders"
          :key="folder.id"
          :active="folder.id === currentFolderId"
          rounded="xl"
          @click="$emit('select-folder', folder.id)"
        >
          <template #prepend>
            <v-icon :icon="folder.icon" />
          </template>
          <v-list-item-title>{{ folder.name }}</v-list-item-title>
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
          {{ accounts.length ? 'Select an account to view its folders.' : 'Add an account to get started.' }}
        </div>
      </div>
    </v-card>
  </div>

  <v-dialog v-model="deleteDialog" max-width="400" persistent>
    <v-card>
      <v-card-title>Delete account</v-card-title>
      <v-card-text>
        Are you sure you want to delete <strong>{{ pendingDeleteAccount?.name }}</strong> ({{ pendingDeleteAccount?.email }})? All local data for this account will be removed.
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn @click="deleteDialog = false">Cancel</v-btn>
        <v-btn color="error" variant="flat" @click="emitDelete">Delete</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { MailAccount } from '@/types/account'
import type { MailboxFolder } from '@/types/mail'

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
