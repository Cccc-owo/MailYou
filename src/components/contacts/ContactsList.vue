<template>
  <div class="contacts-list">
    <div class="contacts-list__header pa-3 d-flex align-center justify-space-between">
      <div>
        <div class="text-subtitle-1 font-weight-medium">{{ title }}</div>
        <div class="text-caption text-medium-emphasis">{{ t('contacts.contactCount', { count: contacts.length }) }}</div>
      </div>
      <div class="d-flex ga-1">
        <v-menu>
          <template #activator="{ props }">
            <v-btn icon="mdi-dots-vertical" size="small" variant="tonal" :title="t('contacts.manage')" v-bind="props" />
          </template>
          <v-list density="compact">
            <v-list-item prepend-icon="mdi-import" :title="t('contacts.importContacts')" @click="$emit('import')" />
            <v-list-item prepend-icon="mdi-export" :title="t('contacts.exportAsVCard')" @click="$emit('exportVCard')" />
            <v-list-item prepend-icon="mdi-file-delimited" :title="t('contacts.exportAsCsv')" @click="$emit('exportCsv')" />
            <v-divider />
            <v-list-item prepend-icon="mdi-merge" :title="t('contacts.mergeDuplicates')" @click="$emit('merge')" />
          </v-list>
        </v-menu>
        <v-btn icon="mdi-plus" size="small" variant="tonal" :title="t('contacts.addContact')" @click="$emit('add')" />
      </div>
    </div>

    <v-divider />

    <v-list class="contacts-list__items" lines="two">
      <v-list-item
        v-for="contact in contacts"
        :key="contact.id"
        :active="contact.id === selectedContactId"
        @click="$emit('select', contact.id)"
      >
        <template #prepend>
          <v-avatar color="primary" size="36">
            <v-img v-if="contactsStore.avatarUrl(contact)" :src="contactsStore.avatarUrl(contact)!" cover />
            <span v-else class="text-body-2">{{ initials(contact.name || contact.email) }}</span>
          </v-avatar>
        </template>
        <v-list-item-title>{{ contact.name || contact.email }}</v-list-item-title>
        <v-list-item-subtitle>{{ contact.email }}</v-list-item-subtitle>
      </v-list-item>

      <div v-if="contacts.length === 0" class="pa-6 text-center text-medium-emphasis">
        <v-icon size="48" icon="mdi-account-outline" class="mb-2" />
        <div class="text-body-2">{{ t('contacts.noContacts') }}</div>
        <div class="text-caption">{{ t('contacts.noContactsHint') }}</div>
      </div>
    </v-list>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Contact } from '@/types/contact'
import { useContactsStore } from '@/stores/contacts'

const { t } = useI18n()
const contactsStore = useContactsStore()

defineProps<{
  contacts: Contact[]
  selectedContactId: string | null
  title: string
}>()

defineEmits<{
  select: [contactId: string]
  add: []
  import: []
  exportVCard: []
  exportCsv: []
  merge: []
}>()

const initials = (name: string) => {
  return name
    .split(/[\s@]/)
    .filter(Boolean)
    .slice(0, 2)
    .map((s) => s[0].toUpperCase())
    .join('')
}
</script>

<style scoped>
.contacts-list {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.contacts-list__items {
  flex: 1;
  overflow-y: auto;
}
</style>
