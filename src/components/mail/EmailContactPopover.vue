<template>
  <v-menu :open-on-hover="!disableHover" :close-on-content-click="false" open-delay="300" location="bottom start">
    <template #activator="{ props: menuProps }">
      <span v-bind="menuProps" class="email-contact-link">
        <slot>{{ name || email }}</slot>
      </span>
    </template>
    <v-card class="popover-card" min-width="280" max-width="360" elevation="8">
      <!-- Header with colored banner -->
      <div class="popover-header">
        <v-avatar size="48" class="popover-avatar" color="primary">
          <v-img v-if="matchedAvatarUrl" :src="matchedAvatarUrl" cover />
          <span v-else class="text-h6" style="color: white;">{{ avatarInitials }}</span>
        </v-avatar>
      </div>

      <v-card-text class="popover-body pt-7 pb-3 px-4">
        <div class="text-subtitle-1 font-weight-bold text-center">{{ displayName }}</div>
        <div class="text-body-2 text-medium-emphasis text-center mb-3">{{ email }}</div>

        <template v-if="matched">
          <div v-if="matched.phone || matched.notes || matchedGroupName" class="popover-details mb-2">
            <div v-if="matched.phone" class="popover-detail-row">
              <v-icon size="16" icon="mdi-phone-outline" color="medium-emphasis" />
              <span>{{ matched.phone }}</span>
            </div>
            <div v-if="matchedGroupName" class="popover-detail-row">
              <v-icon size="16" icon="mdi-label-outline" color="medium-emphasis" />
              <span>{{ matchedGroupName }}</span>
            </div>
            <div v-if="matched.notes" class="popover-detail-row popover-detail-notes">
              <v-icon size="16" icon="mdi-note-text-outline" color="medium-emphasis" />
              <span class="text-medium-emphasis">{{ matched.notes }}</span>
            </div>
          </div>
        </template>

        <v-divider class="mb-2" />

        <div class="d-flex justify-center ga-1">
          <v-btn size="small" variant="tonal" density="comfortable" prepend-icon="mdi-email-outline" @click="$emit('compose', { name: displayName, email })">
            {{ t('contacts.compose') }}
          </v-btn>
          <v-btn v-if="!matched" size="small" variant="tonal" density="comfortable" prepend-icon="mdi-account-plus-outline" @click="$emit('save-contact', { name: name || '', email })">
            {{ t('contacts.saveToContacts') }}
          </v-btn>
          <v-btn v-if="matched" size="small" variant="tonal" density="comfortable" prepend-icon="mdi-account-outline" @click="$emit('view-contact', matched)">
            {{ t('contacts.viewContact') }}
          </v-btn>
        </div>
      </v-card-text>
    </v-card>
  </v-menu>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContactsStore } from '@/stores/contacts'
import type { Contact } from '@/types/contact'

const { t } = useI18n()
const contactsStore = useContactsStore()

const props = withDefaults(
  defineProps<{
    name?: string
    email: string
    disableHover?: boolean
  }>(),
  { name: '', disableHover: false },
)

defineEmits<{
  compose: [data: { name: string; email: string }]
  'save-contact': [data: { name: string; email: string }]
  'view-contact': [contact: Contact]
}>()

const matched = computed(() =>
  contactsStore.contacts.find((c) => c.email.toLowerCase() === props.email.toLowerCase()) ?? null,
)

const matchedAvatarUrl = computed(() => contactsStore.avatarUrl(matched.value))

const matchedGroupName = computed(() => {
  if (!matched.value?.groupId) return null
  return contactsStore.contactGroups.find((g) => g.id === matched.value!.groupId)?.name ?? null
})

const displayName = computed(() =>
  matched.value?.name || props.name || props.email,
)

const avatarInitials = computed(() => {
  const src = displayName.value
  return src
    .split(/[\s@]/)
    .filter(Boolean)
    .slice(0, 2)
    .map((s) => s[0].toUpperCase())
    .join('')
})
</script>

<style scoped>
.email-contact-link {
  cursor: pointer;
  border-radius: 4px;
  padding: 0 2px;
  transition: background-color 0.15s;
}

.email-contact-link:hover {
  background-color: rgba(var(--v-theme-primary), 0.08);
  color: rgb(var(--v-theme-primary));
}

.popover-card {
  overflow: visible;
  border-radius: 12px;
}

.popover-header {
  height: 48px;
  background: linear-gradient(135deg, rgb(var(--v-theme-primary)), rgb(var(--v-theme-primary-darken-1), 1));
  border-radius: 12px 12px 0 0;
  position: relative;
}

.popover-avatar {
  position: absolute;
  bottom: -24px;
  left: 50%;
  transform: translateX(-50%);
  border: 3px solid rgb(var(--v-theme-surface));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.popover-body {
  position: relative;
}

.popover-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: rgba(var(--v-theme-on-surface), 0.03);
  border-radius: 8px;
  padding: 8px 10px;
}

.popover-detail-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 0.8125rem;
  line-height: 1.4;
}

.popover-detail-notes {
  white-space: pre-line;
}
</style>
