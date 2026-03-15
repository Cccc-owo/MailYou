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
              <span>{{ groupSharedEmails(group).join(', ') }} ({{ group.length }})</span>
            </v-list-subheader>

            <v-list-item v-for="(contact, ci) in group" :key="contact.id" class="pl-10">
              <template #prepend>
                <v-avatar color="primary" size="32" class="mr-3">
                  <v-img v-if="contactsStore.avatarUrl(contact)" :src="contactsStore.avatarUrl(contact)!" cover />
                  <span v-else class="text-caption">{{ initials(contact.name || contact.emails[0] || '') }}</span>
                </v-avatar>
              </template>
              <v-list-item-title>
                {{ contact.name || contact.emails[0] || '' }}
                <v-chip v-if="ci === 0" size="x-small" color="primary" class="ml-1">{{ t('contacts.mergeKeep') }}</v-chip>
                <v-chip v-else size="x-small" color="error" variant="outlined" class="ml-1">{{ t('contacts.mergeRemove') }}</v-chip>
              </v-list-item-title>
              <v-list-item-subtitle>
                {{ [contact.phones.join(', '), contact.notes].filter(Boolean).join(' · ') || contact.emails.join(', ') }}
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

// Group contacts that share any email address (union-find style)
const duplicateGroups = computed(() => {
  const emailToContacts = new Map<string, Set<number>>()

  for (let i = 0; i < props.contacts.length; i++) {
    const c = props.contacts[i]
    for (const email of c.emails) {
      const key = email.toLowerCase()
      const set = emailToContacts.get(key)
      if (set) set.add(i)
      else emailToContacts.set(key, new Set([i]))
    }
  }

  // Merge sets that share contacts (transitive grouping)
  const visited = new Set<number>()
  const groups: Contact[][] = []

  for (let i = 0; i < props.contacts.length; i++) {
    if (visited.has(i)) continue
    const group = new Set<number>()
    const queue = [i]
    while (queue.length) {
      const idx = queue.pop()!
      if (group.has(idx)) continue
      group.add(idx)
      visited.add(idx)
      for (const email of props.contacts[idx].emails) {
        const related = emailToContacts.get(email.toLowerCase())
        if (related) {
          for (const r of related) {
            if (!group.has(r)) queue.push(r)
          }
        }
      }
    }
    if (group.size > 1) {
      groups.push(Array.from(group).map((idx) => props.contacts[idx]))
    }
  }

  return groups
})

const groupSharedEmails = (group: Contact[]): string[] => {
  const counts = new Map<string, number>()
  for (const c of group) {
    for (const e of c.emails) {
      const key = e.toLowerCase()
      counts.set(key, (counts.get(key) ?? 0) + 1)
    }
  }
  return Array.from(counts.entries())
    .filter(([, count]) => count > 1)
    .map(([email]) => email)
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
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

    // Union all emails, deduplicate
    const allEmails = new Set(primary.emails.map((e) => e.toLowerCase()))
    const emailsList = [...primary.emails]
    const allPhones = new Set(primary.phones.map((p) => p.replace(/\s/g, '')))
    const phonesList = [...primary.phones]

    for (const other of group) {
      if (other.id === primary.id) continue
      if (!primary.name && other.name) primary.name = other.name
      if (!primary.notes && other.notes) primary.notes = other.notes
      if (!primary.avatarPath && other.avatarPath) primary.avatarPath = other.avatarPath
      if (!primary.groupId && other.groupId) primary.groupId = other.groupId
      for (const e of other.emails) {
        if (!allEmails.has(e.toLowerCase())) {
          allEmails.add(e.toLowerCase())
          emailsList.push(e)
        }
      }
      for (const p of other.phones) {
        const normalized = p.replace(/\s/g, '')
        if (!allPhones.has(normalized)) {
          allPhones.add(normalized)
          phonesList.push(p)
        }
      }
    }

    primary.emails = emailsList
    primary.phones = phonesList

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
