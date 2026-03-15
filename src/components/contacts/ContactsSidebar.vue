<template>
  <div class="contacts-sidebar pa-3">
    <div class="d-flex align-center justify-space-between mb-2">
      <span class="text-subtitle-2 text-medium-emphasis">{{ t('contacts.groups') }}</span>
      <v-btn icon="mdi-plus" size="x-small" variant="text" :title="t('contacts.addGroup')" @click="emit('add-group')" />
    </div>

    <v-list density="compact" nav>
      <v-list-item
        :active="currentGroupId === null"
        prepend-icon="mdi-account-group-outline"
        :title="t('contacts.allContacts')"
        :subtitle="String(contacts.length)"
        @click="emit('select-group', null)"
      />
      <v-list-item
        v-for="group in groups"
        :key="group.id"
        :active="currentGroupId === group.id"
        prepend-icon="mdi-label-outline"
        :title="group.name"
        :subtitle="String(contacts.filter(c => c.groupId === group.id).length)"
        @click="emit('select-group', group.id)"
        @contextmenu="ctx.open($event, group)"
      />
    </v-list>

    <ContextMenu v-model="ctx.isOpen.value" :x="ctx.x.value" :y="ctx.y.value">
      <v-list-item prepend-icon="mdi-pencil-outline" :title="t('contacts.editGroup')" @click="emit('rename-group', ctx.target.value!)" />
      <v-list-item prepend-icon="mdi-delete-outline" :title="t('contacts.deleteGroup')" @click="emit('delete-group', ctx.target.value!.id)" />
    </ContextMenu>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import ContextMenu from '@/components/ContextMenu.vue'
import { useContextMenu } from '@/composables/useContextMenu'
import type { Contact, ContactGroup } from '@/types/contact'

const { t } = useI18n()
const ctx = useContextMenu<ContactGroup>()

defineProps<{
  groups: ContactGroup[]
  contacts: Contact[]
  currentGroupId: string | null
}>()

const emit = defineEmits<{
  'select-group': [groupId: string | null]
  'add-group': []
  'rename-group': [group: ContactGroup]
  'delete-group': [groupId: string]
}>()
</script>
