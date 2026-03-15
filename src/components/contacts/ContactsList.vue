<template>
  <div class="contacts-list">
    <div class="contacts-list__header pa-3 d-flex align-center justify-space-between">
      <div>
        <div class="text-subtitle-1 font-weight-medium">{{ title }}</div>
        <div class="text-caption text-medium-emphasis">{{ t('contacts.contactCount', { count: contacts.length }) }}</div>
      </div>
      <v-btn icon="mdi-plus" size="small" variant="tonal" :title="t('contacts.addContact')" @click="$emit('add')" />
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
