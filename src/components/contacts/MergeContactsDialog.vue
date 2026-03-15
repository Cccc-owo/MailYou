<template>
  <v-dialog :model-value="modelValue" max-width="600" @update:model-value="$emit('update:modelValue', $event)">
    <v-card>
      <v-card-title>{{ t('contacts.mergeDialogTitle') }}</v-card-title>

      <v-card-text v-if="duplicateGroups.length === 0" class="text-center pa-6">
        <v-icon size="48" icon="mdi-check-circle-outline" color="success" class="mb-2" />
        <div class="text-body-1">{{ t('contacts.noDuplicates') }}</div>
      </v-card-text>

      <v-card-text v-else class="pa-0">
        <v-list lines="three">
          <template v-for="(group, gi) in duplicateGroups" :key="gi">
            <v-list-subheader class="d-flex align-center">
              <v-checkbox-btn
                :model-value="selectedGroups.has(gi)"
                density="compact"
                class="mr-2"
                @update:model-value="toggleGroup(gi, $event)"
              />
              <span>{{ group[0].email }} ({{ group.length }})</span>
            </v-list-subheader>

            <v-list-item v-for="(contact, ci) in group" :key="contact.id" class="pl-10">
              <template #prepend>
                <v-avatar color="primary" size="32" class="mr-3">
                  <v-img v-if="contactsStore.avatarUrl(contact)" :src="contactsStore.avatarUrl(contact)!" cover />
                  <span v-else class="text-caption">{{ initials(contact.name || contact.email) }}</span>
                </v-avatar>
              </template>
              <v-list-item-title>
                {{ contact.name || contact.email }}
                <v-chip v-if="ci === 0" size="x-small" color="primary" class="ml-1">{{ t('contacts.mergeKeep') }}</v-chip>
                <v-chip v-else size="x-small" color="error" variant="outlined" class="ml-1">{{ t('contacts.mergeRemove') }}</v-chip>
              </v-list-item-title>
              <v-list-item-subtitle>
                {{ [contact.phone, contact.notes].filter(Boolean).join(' · ') || contact.email }}
              </v-list-item-subtitle>
            </v-list-item>

            <v-divider v-if="gi < duplicateGroups.length - 1" />
          </template>
        </v-list>
      </v-card-text>

      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="$emit('update:modelValue', false)">{{ t('common.cancel') }}</v-btn>
        <v-btn
          v-if="duplicateGroups.length > 0"
          color="primary"
          :disabled="selectedGroups.size === 0"
          @click="confirmMerge"
        >
          {{ t('contacts.mergeConfirm') }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Contact } from '@/types/contact'
import { useContactsStore } from '@/stores/contacts'

const { t } = useI18n()
const contactsStore = useContactsStore()

const props = defineProps<{
  modelValue: boolean
  contacts: Contact[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  merged: [count: number]
}>()

const selectedGroups = ref(new Set<number>())

const duplicateGroups = computed(() => {
  const groups = new Map<string, Contact[]>()
  for (const c of props.contacts) {
    if (!c.email) continue
    const key = c.email.toLowerCase()
    const list = groups.get(key)
    if (list) list.push(c)
    else groups.set(key, [c])
  }
  return Array.from(groups.values()).filter((g) => g.length > 1)
})

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      // Select all groups by default
      selectedGroups.value = new Set(duplicateGroups.value.map((_, i) => i))
    }
  },
)

const toggleGroup = (index: number, checked: boolean) => {
  const next = new Set(selectedGroups.value)
  if (checked) next.add(index)
  else next.delete(index)
  selectedGroups.value = next
}

const confirmMerge = async () => {
  let mergedCount = 0

  for (const gi of selectedGroups.value) {
    const group = duplicateGroups.value[gi]
    if (!group || group.length < 2) continue

    // Pick the primary: prefer one with an avatar
    const primaryIdx = group.findIndex((c) => c.avatarPath) >= 0
      ? group.findIndex((c) => c.avatarPath)
      : 0
    const primary = { ...group[primaryIdx] }

    // Fill empty fields from others
    for (const other of group) {
      if (other.id === primary.id) continue
      if (!primary.name && other.name) primary.name = other.name
      if (!primary.phone && other.phone) primary.phone = other.phone
      if (!primary.notes && other.notes) primary.notes = other.notes
      if (!primary.avatarPath && other.avatarPath) primary.avatarPath = other.avatarPath
      if (!primary.groupId && other.groupId) primary.groupId = other.groupId
    }

    // Update primary
    await contactsStore.updateContact(primary.id, primary as Contact)

    // Delete others
    for (const other of group) {
      if (other.id === primary.id) continue
      await contactsStore.deleteContact(other.id)
    }

    mergedCount++
  }

  emit('merged', mergedCount)
  emit('update:modelValue', false)
}

const initials = (name: string) => {
  return name
    .split(/[\s@]/)
    .filter(Boolean)
    .slice(0, 2)
    .map((s) => s[0].toUpperCase())
    .join('')
}
</script>
