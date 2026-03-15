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
        @import="handleImport"
        @export-v-card="handleExportVCard"
        @export-csv="handleExportCsv"
        @merge="handleMerge"
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

  <MergeContactsDialog
    v-model="mergeDialogOpen"
    :contacts="contactsStore.contacts"
    @merged="handleMerged"
  />
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import MailShellLayout from '@/layouts/MailShellLayout.vue'
import ContactsSidebar from '@/components/contacts/ContactsSidebar.vue'
import ContactsList from '@/components/contacts/ContactsList.vue'
import ContactDetail from '@/components/contacts/ContactDetail.vue'
import MergeContactsDialog from '@/components/contacts/MergeContactsDialog.vue'
import { useContactsStore } from '@/stores/contacts'
import { useComposerStore } from '@/stores/composer'
import { useAccountsStore } from '@/stores/accounts'
import { parseVCard, generateVCard, parseCsv, generateCsv } from '@/utils/contactIO'
import type { Contact, ContactGroup } from '@/types/contact'

const { t } = useI18n()
const router = useRouter()
const contactsStore = useContactsStore()
const composerStore = useComposerStore()
const accountsStore = useAccountsStore()

const searchQuery = ref('')
const snackbar = ref(false)
const snackbarText = ref('')
const mergeDialogOpen = ref(false)

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
  return base.filter((c) => c.name.toLowerCase().includes(q) || c.emails.some((e) => e.toLowerCase().includes(q)))
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
  composerStore.draft.to = `${contact.name} <${contact.emails[0] || ''}>`
}

// Import / Export / Merge
const handleImport = async () => {
  const result = await window.windowControls?.openTextFile([
    { name: 'Contacts', extensions: ['vcf', 'csv'] },
  ])
  if (!result) return

  const ext = result.fileName.split('.').pop()?.toLowerCase()
  const parsed = ext === 'csv' ? parseCsv(result.content) : parseVCard(result.content)

  let count = 0
  for (const c of parsed) {
    await contactsStore.createContact(c)
    count++
  }
  showSnackbar(t('contacts.importSuccess', { count }))
}

const handleExportVCard = async () => {
  const content = generateVCard(contactsStore.contacts)
  const ok = await window.windowControls?.saveTextFile(content, 'contacts.vcf', [
    { name: 'vCard', extensions: ['vcf'] },
  ])
  if (ok) showSnackbar(t('contacts.exportSuccess'))
}

const handleExportCsv = async () => {
  const content = generateCsv(contactsStore.contacts)
  const ok = await window.windowControls?.saveTextFile(content, 'contacts.csv', [
    { name: 'CSV', extensions: ['csv'] },
  ])
  if (ok) showSnackbar(t('contacts.exportSuccess'))
}

const handleMerge = () => {
  mergeDialogOpen.value = true
}

const handleMerged = (count: number) => {
  showSnackbar(t('contacts.mergeSuccess', { count }))
}

onMounted(async () => {
  await Promise.all([contactsStore.loadContacts(), contactsStore.loadGroups(), contactsStore.ensureStorageDir()])
})
</script>
