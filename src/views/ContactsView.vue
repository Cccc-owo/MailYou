<template>
  <MailShellLayout :search="searchQuery" @update:search="handleSearch" :search-placeholder="t('contacts.searchPlaceholder')">
    <template #actions>
      <v-btn prepend-icon="mdi-arrow-left" variant="text" @click="router.push('/')">{{ t('common.backToMail') }}</v-btn>
    </template>

    <template #sidebar>
      <ContactsSidebar
        :groups="contactsStore.contactGroups"
        :contacts="contactsStore.contacts"
        :current-group-id="contactsStore.currentGroupId"
        @select-group="handleSelectGroup"
        @add-group="openGroupDialog(null)"
        @rename-group="openGroupDialog"
        @delete-group="handleDeleteGroup"
      />
    </template>

    <template #list>
      <ContactsList
        :contacts="displayContacts"
        :selected-contact-id="contactsStore.selectedContactId"
        :title="currentGroupName"
        @select="contactsStore.selectContact"
        @add="contactsStore.startCreate()"
      />
    </template>

    <template #reader>
      <ContactDetail
        :contact="contactsStore.selectedContact"
        :groups="contactsStore.contactGroups"
        :is-editing="contactsStore.isEditing"
        :is-creating="contactsStore.isCreating"
        :default-group-id="contactsStore.currentGroupId"
        @update:is-editing="contactsStore.isEditing = $event"
        @update:is-creating="contactsStore.isCreating = $event"
        @compose="composeToContact"
        @delete="handleDeleteContact"
        @save="handleSaveContact"
        @create="handleCreateContact"
      />
    </template>
  </MailShellLayout>

  <!-- Group name dialog -->
  <v-dialog v-model="groupDialog.open" max-width="360">
    <v-card>
      <v-card-title>{{ groupDialog.groupId ? t('contacts.editGroup') : t('contacts.addGroup') }}</v-card-title>
      <v-card-text>
        <v-text-field
          v-model="groupDialog.name"
          :label="t('contacts.groupNamePlaceholder')"
          autofocus
          @keydown.enter="confirmGroupDialog"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="groupDialog.open = false">{{ t('common.cancel') }}</v-btn>
        <v-btn color="primary" :disabled="!groupDialog.name.trim()" @click="confirmGroupDialog">{{ t('common.save') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-snackbar v-model="snackbar" location="bottom right" color="primary">
    {{ snackbarText }}
  </v-snackbar>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import MailShellLayout from '@/layouts/MailShellLayout.vue'
import ContactsSidebar from '@/components/contacts/ContactsSidebar.vue'
import ContactsList from '@/components/contacts/ContactsList.vue'
import ContactDetail from '@/components/contacts/ContactDetail.vue'
import { useContactsStore } from '@/stores/contacts'
import { useComposerStore } from '@/stores/composer'
import { useAccountsStore } from '@/stores/accounts'
import type { Contact, ContactGroup } from '@/types/contact'

const { t } = useI18n()
const router = useRouter()
const contactsStore = useContactsStore()
const composerStore = useComposerStore()
const accountsStore = useAccountsStore()

const searchQuery = ref('')
const snackbar = ref(false)
const snackbarText = ref('')

const groupDialog = reactive({
  open: false,
  groupId: null as string | null,
  name: '',
})

const showSnackbar = (text: string) => {
  snackbarText.value = text
  snackbar.value = true
}

const currentGroupName = computed(() => {
  if (!contactsStore.currentGroupId) return t('contacts.allContacts')
  return contactsStore.contactGroups.find((g) => g.id === contactsStore.currentGroupId)?.name ?? t('contacts.allContacts')
})

const displayContacts = computed(() => {
  const base = contactsStore.filteredContacts
  if (!searchQuery.value.trim()) return base
  const q = searchQuery.value.toLowerCase()
  return base.filter((c) => c.name.toLowerCase().includes(q) || c.email.toLowerCase().includes(q))
})

const handleSearch = (val: string) => {
  searchQuery.value = val
}

const handleSelectGroup = (groupId: string | null) => {
  contactsStore.selectGroup(groupId)
}

// Group dialog
const openGroupDialog = (group: ContactGroup | null) => {
  groupDialog.groupId = group?.id ?? null
  groupDialog.name = group?.name ?? ''
  groupDialog.open = true
}

const confirmGroupDialog = async () => {
  const name = groupDialog.name.trim()
  if (!name) return
  if (groupDialog.groupId) {
    await contactsStore.updateGroup(groupDialog.groupId, name)
  } else {
    await contactsStore.createGroup(name)
  }
  groupDialog.open = false
}

const handleDeleteGroup = async (groupId: string) => {
  await contactsStore.deleteGroup(groupId)
}

// Contact actions
const handleCreateContact = async (contact: Partial<Contact>) => {
  const created = await contactsStore.createContact(contact)
  contactsStore.selectContact(created.id)
  showSnackbar(t('contacts.contactSaved'))
}

const handleSaveContact = async (contact: Contact) => {
  await contactsStore.updateContact(contact.id, contact)
  showSnackbar(t('contacts.contactSaved'))
}

const handleDeleteContact = async (contactId: string) => {
  await contactsStore.deleteContact(contactId)
  showSnackbar(t('contacts.contactDeleted'))
}

const composeToContact = (contact: Contact) => {
  const accountId = accountsStore.currentAccountId
  if (!accountId) return
  composerStore.openNew(accountId)
  composerStore.draft.to = `${contact.name} <${contact.email}>`
}

onMounted(async () => {
  await Promise.all([contactsStore.loadContacts(), contactsStore.loadGroups(), contactsStore.ensureStorageDir()])
})
</script>
