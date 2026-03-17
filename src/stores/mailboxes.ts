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
  const error = ref<string | null>(null)

  const currentFolder = computed(() =>
    folders.value.find((folder) => folder.id === currentFolderId.value) ?? null,
  )

  const setFolders = (nextFolders: MailboxFolder[]) => {
    folders.value = nextFolders
    currentFolderId.value = getDefaultFolderId(nextFolders, currentFolderId.value)
  }

  const loadFolders = async (accountId: string) => {
    isLoading.value = true
    error.value = null

    try {
      setFolders(await mailRepository.listFolders(accountId))
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load folders'
      throw loadError
    } finally {
      isLoading.value = false
    }
  }

  const createFolder = async (accountId: string, name: string) => {
    error.value = null
    try {
      const nextFolders = await mailRepository.createFolder(accountId, name)
      setFolders(nextFolders)
      return nextFolders
    } catch (folderError) {
      error.value = folderError instanceof Error ? folderError.message : 'Unable to create folder'
      throw folderError
    }
  }

  const renameFolder = async (accountId: string, folderId: string, name: string) => {
    error.value = null
    try {
      const nextFolders = await mailRepository.renameFolder(accountId, folderId, name)
      setFolders(nextFolders)
      return nextFolders
    } catch (folderError) {
      error.value = folderError instanceof Error ? folderError.message : 'Unable to rename folder'
      throw folderError
    }
  }

  const deleteFolder = async (accountId: string, folderId: string) => {
    error.value = null
    try {
      const nextFolders = await mailRepository.deleteFolder(accountId, folderId)
      setFolders(nextFolders)
      return nextFolders
    } catch (folderError) {
      error.value = folderError instanceof Error ? folderError.message : 'Unable to delete folder'
      throw folderError
    }
  }

  const selectFolder = (folderId: string) => {
    currentFolderId.value = folderId
  }

  const decrementUnread = (folderId: string) => {
    const folder = folders.value.find((f) => f.id === folderId)
    if (folder && folder.unreadCount > 0) {
      folder.unreadCount--
    }
  }

  const incrementUnread = (folderId: string) => {
    const folder = folders.value.find((f) => f.id === folderId)
    if (folder) {
      folder.unreadCount++
    }
  }

  const adjustUnread = (folderId: string, delta: number) => {
    if (delta === 0) {
      return
    }

    const folder = folders.value.find((f) => f.id === folderId)
    if (folder) {
      folder.unreadCount = Math.max(0, folder.unreadCount + delta)
    }
  }

  return {
    folders,
    currentFolder,
    currentFolderId,
    isLoading,
    error,
    setFolders,
    loadFolders,
    createFolder,
    renameFolder,
    deleteFolder,
    selectFolder,
    decrementUnread,
    incrementUnread,
    adjustUnread,
  }
})
