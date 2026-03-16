import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import type { Contact, ContactGroup } from '@/types/contact'

export const useContactsStore = defineStore('contacts', () => {
  const contacts = ref<Contact[]>([])
  const contactGroups = ref<ContactGroup[]>([])
  const currentGroupId = ref<string | null>(null)
  const selectedContactId = ref<string | null>(null)
  const isLoading = ref(false)
  const isEditing = ref(false)
  const isCreating = ref(false)
  const error = ref<string | null>(null)

  const selectedContact = computed(() =>
    contacts.value.find((c) => c.id === selectedContactId.value) ?? null,
  )

  const filteredContacts = computed(() => {
    if (!currentGroupId.value) return contacts.value
    return contacts.value.filter((c) => c.groupId === currentGroupId.value)
  })

  const loadContacts = async (groupId?: string) => {
    isLoading.value = true
    error.value = null
    try {
      contacts.value = await mailRepository.listContacts(groupId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to load contacts'
    } finally {
      isLoading.value = false
    }
  }

  const loadGroups = async () => {
    try {
      contactGroups.value = await mailRepository.listContactGroups()
    } catch {
      // silent
    }
  }

  const createContact = async (contact: Partial<Contact>) => {
    const full: Contact = {
      id: '',
      name: contact.name ?? '',
      emails: contact.emails?.length ? contact.emails : [],
      phones: contact.phones?.length ? contact.phones : [],
      notes: contact.notes,
      groupId: contact.groupId,
      sourceAccountId: contact.sourceAccountId,
      createdAt: '',
      updatedAt: '',
    }
    const created = await mailRepository.createContact(full)
    contacts.value.push(created)
    contacts.value.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
    return created
  }

  const updateContact = async (contactId: string, contact: Contact) => {
    const updated = await mailRepository.updateContact(contactId, contact)
    const idx = contacts.value.findIndex((c) => c.id === contactId)
    if (idx >= 0) contacts.value[idx] = updated
    return updated
  }

  const deleteContact = async (contactId: string) => {
    await mailRepository.deleteContact(contactId)
    contacts.value = contacts.value.filter((c) => c.id !== contactId)
    if (selectedContactId.value === contactId) selectedContactId.value = null
  }

  const searchContacts = async (query: string): Promise<Contact[]> => {
    return mailRepository.searchContacts(query)
  }

  const createGroup = async (name: string) => {
    const group = await mailRepository.createContactGroup(name)
    contactGroups.value.push(group)
    return group
  }

  const updateGroup = async (groupId: string, name: string) => {
    const updated = await mailRepository.updateContactGroup(groupId, name)
    const idx = contactGroups.value.findIndex((g) => g.id === groupId)
    if (idx >= 0) contactGroups.value[idx] = updated
    return updated
  }

  const deleteGroup = async (groupId: string) => {
    await mailRepository.deleteContactGroup(groupId)
    contactGroups.value = contactGroups.value.filter((g) => g.id !== groupId)
    if (currentGroupId.value === groupId) currentGroupId.value = null
    // Unlink contacts client-side
    for (const c of contacts.value) {
      if (c.groupId === groupId) c.groupId = undefined
    }
  }

  const selectGroup = (groupId: string | null) => {
    currentGroupId.value = groupId
    selectedContactId.value = null
  }

  const selectContact = (contactId: string) => {
    selectedContactId.value = contactId
    isEditing.value = false
    isCreating.value = false
  }

  const startCreate = () => {
    selectedContactId.value = null
    isCreating.value = true
    isEditing.value = true
  }

  const avatarUrl = (contact: Contact | null | undefined): string | null => {
    if (!contact?.avatarPath) return null
    return `mailyou-avatar://${contact.avatarPath}`
  }

  const uploadAvatar = async (contactId: string, dataBase64: string, mimeType: string) => {
    const updated = await mailRepository.uploadContactAvatar(contactId, dataBase64, mimeType)
    const idx = contacts.value.findIndex((c) => c.id === contactId)
    if (idx >= 0) contacts.value[idx] = updated
    return updated
  }

  const deleteAvatar = async (contactId: string) => {
    const updated = await mailRepository.deleteContactAvatar(contactId)
    const idx = contacts.value.findIndex((c) => c.id === contactId)
    if (idx >= 0) contacts.value[idx] = updated
    return updated
  }

  return {
    contacts,
    contactGroups,
    currentGroupId,
    selectedContactId,
    selectedContact,
    filteredContacts,
    isLoading,
    isEditing,
    isCreating,
    error,
    loadContacts,
    loadGroups,
    createContact,
    updateContact,
    deleteContact,
    searchContacts,
    createGroup,
    updateGroup,
    deleteGroup,
    selectGroup,
    selectContact,
    startCreate,
    avatarUrl,
    uploadAvatar,
    deleteAvatar,
  }
})
