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

      <v-list class="mail-sidebar__accounts" lines="two">
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
          </template>
        </v-list-item>
      </v-list>
    </v-card>

    <v-card class="mail-sidebar__panel" color="surface">
      <div class="mail-sidebar__header d-flex align-center justify-space-between ga-3 flex-wrap">
        <div>
          <div class="text-overline">Folders</div>
          <div class="text-h6">{{ currentAccount?.name ?? 'Select account' }}</div>
        </div>
        <v-btn prepend-icon="mdi-pencil-plus-outline" @click="$emit('compose')">Compose</v-btn>
      </div>

      <v-list nav>
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
            <span class="text-caption text-medium-emphasis">{{ folder.unreadCount }}</span>
          </template>
        </v-list-item>
      </v-list>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import type { MailAccount } from '@/types/account'
import type { MailboxFolder } from '@/types/mail'

defineProps<{
  accounts: MailAccount[]
  currentAccount: MailAccount | null
  currentAccountId: string | null
  currentFolderId: string | null
  folders: MailboxFolder[]
}>()

defineEmits<{
  'select-account': [accountId: string]
  'select-folder': [folderId: string]
  compose: []
  'add-account': []
}>()
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
