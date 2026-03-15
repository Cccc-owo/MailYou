<template>
  <div v-if="!contact && !isCreating" class="d-flex flex-column align-center justify-center fill-height text-medium-emphasis pa-6">
    <v-icon size="64" icon="mdi-account-circle-outline" class="mb-3" />
    <div class="text-h6">{{ t('contacts.noContactSelected') }}</div>
    <div class="text-body-2">{{ t('contacts.selectContactHint') }}</div>
  </div>

  <div v-else class="contact-detail pa-4">
    <!-- View mode -->
    <template v-if="!isEditing && contact">
      <div class="d-flex align-center mb-4">
        <div class="contact-detail__avatar-wrap mr-4" @click="triggerAvatarPicker">
          <v-avatar color="primary" size="56">
            <v-img v-if="avatarSrc" :src="avatarSrc" cover />
            <span v-else class="text-h6">{{ initials(contact.name || contact.emails[0] || '') }}</span>
          </v-avatar>
          <div class="contact-detail__avatar-overlay">
            <v-icon size="20" icon="mdi-camera" color="white" />
          </div>
        </div>
        <div>
          <div class="text-h6">{{ contact.name || contact.emails[0] || '' }}</div>
          <div v-for="(email, i) in contact.emails" :key="i" class="text-body-2 text-medium-emphasis">{{ email }}</div>
        </div>
      </div>

      <v-list lines="two" density="compact">
        <v-list-item v-for="(phone, i) in contact.phones" :key="'ph'+i" prepend-icon="mdi-phone-outline" :title="t('contacts.phone')" :subtitle="phone" />
        <v-list-item v-if="contact.notes" prepend-icon="mdi-note-text-outline" :title="t('contacts.notes')" :subtitle="contact.notes" />
        <v-list-item v-if="groupName" prepend-icon="mdi-label-outline" :title="t('contacts.group')" :subtitle="groupName" />
      </v-list>

      <div class="d-flex ga-2 mt-4">
        <v-btn prepend-icon="mdi-email-outline" variant="tonal" color="primary" @click="$emit('compose', contact)">{{ t('contacts.compose') }}</v-btn>
        <v-btn prepend-icon="mdi-pencil-outline" variant="text" @click="startEdit">{{ t('common.edit') }}</v-btn>
        <v-btn v-if="avatarSrc" prepend-icon="mdi-image-remove" variant="text" @click="handleDeleteAvatar">{{ t('contacts.removeAvatar') }}</v-btn>
        <v-btn prepend-icon="mdi-delete-outline" variant="text" color="error" @click="$emit('delete', contact.id)">{{ t('common.delete') }}</v-btn>
      </div>
    </template>

    <!-- Edit / Create mode -->
    <template v-if="isEditing">
      <div class="text-h6 mb-4">{{ isCreating ? t('contacts.addContact') : t('contacts.editContact') }}</div>
      <div class="d-flex flex-column ga-3">
        <v-text-field v-model="form.name" :label="t('contacts.name')" autofocus />

        <div v-for="(_, i) in form.emails" :key="i" class="d-flex align-center ga-2">
          <v-text-field
            v-model="form.emails[i]"
            :label="t('contacts.email') + (form.emails.length > 1 ? ` ${i + 1}` : '')"
            :rules="emailRules"
            density="compact"
          />
          <v-btn
            v-if="form.emails.length > 1"
            icon="mdi-close"
            size="small"
            variant="text"
            @click="form.emails.splice(i, 1)"
          />
        </div>
        <v-btn variant="text" size="small" prepend-icon="mdi-plus" class="align-self-start" @click="form.emails.push('')">
          {{ t('contacts.addEmail') }}
        </v-btn>

        <div v-for="(_, i) in form.phones" :key="'ph'+i" class="d-flex align-center ga-2">
          <v-text-field
            v-model="form.phones[i]"
            :label="t('contacts.phone') + (form.phones.length > 1 ? ` ${i + 1}` : '')"
            density="compact"
          />
          <v-btn
            v-if="form.phones.length > 1"
            icon="mdi-close"
            size="small"
            variant="text"
            @click="form.phones.splice(i, 1)"
          />
        </div>
        <v-btn variant="text" size="small" prepend-icon="mdi-plus" class="align-self-start" @click="form.phones.push('')">
          {{ t('contacts.addPhone') }}
        </v-btn>

        <v-textarea v-model="form.notes" :label="t('contacts.notes')" rows="3" />
        <v-select
          v-model="form.groupId"
          :items="groupItems"
          :label="t('contacts.group')"
          clearable
        />
      </div>
      <div class="d-flex ga-2 mt-4">
        <v-btn color="primary" :disabled="!isEmailValid" @click="saveEdit">{{ t('common.save') }}</v-btn>
        <v-btn variant="text" @click="cancelEdit">{{ t('common.cancel') }}</v-btn>
      </div>
    </template>

    <input ref="fileInput" type="file" accept="image/*" class="d-none" @change="handleFileSelected" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Contact, ContactGroup } from '@/types/contact'
import { useContactsStore } from '@/stores/contacts'

const { t } = useI18n()
const contactsStore = useContactsStore()
const fileInput = ref<HTMLInputElement | null>(null)

const props = defineProps<{
  contact: Contact | null
  groups: ContactGroup[]
  isEditing: boolean
  isCreating: boolean
  defaultGroupId?: string | null
}>()

const emit = defineEmits<{
  compose: [contact: Contact]
  delete: [contactId: string]
  save: [contact: Contact]
  create: [contact: Partial<Contact>]
  'update:isEditing': [value: boolean]
  'update:isCreating': [value: boolean]
}>()

const isEditing = computed({
  get: () => props.isEditing,
  set: (v) => emit('update:isEditing', v),
})

const isCreating = computed({
  get: () => props.isCreating,
  set: (v) => emit('update:isCreating', v),
})

const form = ref({ name: '', emails: [''] as string[], phones: [''] as string[], notes: '', groupId: null as string | null })

const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
const emailRules = [
  (v: string) => !!v.trim() || t('contacts.invalidEmail'),
  (v: string) => emailRegex.test(v) || t('contacts.invalidEmail'),
]
const isEmailValid = computed(() =>
  form.value.emails.length > 0 &&
  form.value.emails.every((e) => emailRules.every((rule) => rule(e) === true)),
)

const avatarSrc = computed(() => contactsStore.avatarUrl(props.contact))

const groupItems = computed(() =>
  props.groups.map((g) => ({ title: g.name, value: g.id })),
)

const groupName = computed(() =>
  props.groups.find((g) => g.id === props.contact?.groupId)?.name ?? null,
)

const triggerAvatarPicker = () => {
  fileInput.value?.click()
}

const handleFileSelected = async (event: Event) => {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || !props.contact) return

  const img = new Image()
  img.onload = async () => {
    const canvas = document.createElement('canvas')
    canvas.width = img.width
    canvas.height = img.height
    const ctx = canvas.getContext('2d')!
    ctx.drawImage(img, 0, 0)
    const webpDataUrl = canvas.toDataURL('image/webp', 0.9)
    const base64 = webpDataUrl.split(',')[1]
    if (!base64 || !props.contact) return
    await contactsStore.uploadAvatar(props.contact.id, base64, 'image/webp')
    URL.revokeObjectURL(img.src)
  }
  img.src = URL.createObjectURL(file)
  // Reset so the same file can be re-selected
  input.value = ''
}

const handleDeleteAvatar = async () => {
  if (!props.contact) return
  await contactsStore.deleteAvatar(props.contact.id)
}

const startEdit = () => {
  if (!props.contact) return
  form.value = {
    name: props.contact.name,
    emails: props.contact.emails.length ? [...props.contact.emails] : [''],
    phones: props.contact.phones.length ? [...props.contact.phones] : [''],
    notes: props.contact.notes ?? '',
    groupId: props.contact.groupId ?? null,
  }
  isEditing.value = true
}

const saveEdit = () => {
  const emails = form.value.emails.filter((e) => e.trim())
  const phones = form.value.phones.filter((p) => p.trim())
  if (props.isCreating) {
    emit('create', {
      name: form.value.name,
      emails,
      phones,
      notes: form.value.notes || undefined,
      groupId: form.value.groupId ?? undefined,
    })
  } else if (props.contact) {
    emit('save', {
      ...props.contact,
      name: form.value.name,
      emails,
      phones,
      notes: form.value.notes || undefined,
      groupId: form.value.groupId ?? undefined,
    })
  }
  isEditing.value = false
  isCreating.value = false
}

const cancelEdit = () => {
  isEditing.value = false
  isCreating.value = false
}

// Reset form when entering create mode
watch(
  () => props.isCreating,
  (creating) => {
    if (creating) {
      form.value = { name: '', emails: [''], phones: [''], notes: '', groupId: props.defaultGroupId ?? null }
    }
  },
)

// Reset editing when switching contacts
watch(
  () => props.contact?.id,
  () => {
    if (!props.isCreating) {
      isEditing.value = false
    }
  },
)

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
.contact-detail__avatar-wrap {
  position: relative;
  cursor: pointer;
  border-radius: 50%;
}

.contact-detail__avatar-overlay {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
  opacity: 0;
  transition: opacity 0.2s;
}

.contact-detail__avatar-wrap:hover .contact-detail__avatar-overlay {
  opacity: 1;
}
</style>
