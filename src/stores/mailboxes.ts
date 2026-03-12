import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { MailboxFolder } from '@/types/mail'
import { mailRepository } from '@/services/mail'

const getDefaultFolderId = (folders: MailboxFolder[], currentFolderId: string | null) => {
  if (folders.some((folder) => folder.id === currentFolderId)) {
    return currentFolderId
  }

  return folders.find((folder) => folder.kind === 'inbox')?.id ?? folders[0]?.id ?? null
}

export const useMailboxesStore = defineStore('mailboxes', () => {
  const folders = ref<MailboxFolder[]>([])
  const currentFolderId = ref<string | null>(null)
  const isLoading = ref(false)

  const currentFolder = computed(() =>
    folders.value.find((folder) => folder.id === currentFolderId.value) ?? null,
  )

  const setFolders = (nextFolders: MailboxFolder[]) => {
    folders.value = nextFolders
    currentFolderId.value = getDefaultFolderId(nextFolders, currentFolderId.value)
  }

  const loadFolders = async (accountId: string) => {
    isLoading.value = true

    try {
      setFolders(await mailRepository.listFolders(accountId))
    } finally {
      isLoading.value = false
    }
  }

  const selectFolder = (folderId: string) => {
    currentFolderId.value = folderId
  }

  return {
    folders,
    currentFolder,
    currentFolderId,
    isLoading,
    setFolders,
    loadFolders,
    selectFolder,
  }
})
