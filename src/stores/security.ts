import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import type { StorageSecurityStatus } from '@/types/security'

export const useSecurityStore = defineStore('security', () => {
  const status = ref<StorageSecurityStatus | null>(null)
  const isReady = ref(false)
  const isBusy = ref(false)
  const error = ref<string | null>(null)
  const missingStorageKeyMarker = 'Missing storage key in system keyring'

  const requiresUnlock = computed(() =>
    Boolean(status.value?.hasMasterPassword && !status.value?.isUnlocked),
  )
  const hasMissingStorageKey = computed(() =>
    Boolean(
      status.value
      && !status.value.hasMasterPassword
      && status.value.keyringError?.includes(missingStorageKeyMarker),
    ),
  )
  const hasKeyringIssue = computed(() =>
    Boolean(
      status.value
      && !status.value.hasMasterPassword
      && (!status.value.keyringAvailable || hasMissingStorageKey.value),
    ),
  )

  const refreshStatus = async () => {
    status.value = await mailRepository.getSecurityStatus()
    isReady.value = true
    return status.value
  }

  const initialize = async () => {
    isBusy.value = true
    error.value = null
    try {
      await refreshStatus()
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to load storage security status'
      throw err
    } finally {
      isBusy.value = false
    }
  }

  const unlock = async (password: string) => {
    isBusy.value = true
    error.value = null
    try {
      status.value = await mailRepository.unlockStorage(password)
      return status.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to unlock storage'
      throw err
    } finally {
      isBusy.value = false
    }
  }

  const setMasterPassword = async (currentPassword: string | null, newPassword: string) => {
    isBusy.value = true
    error.value = null
    try {
      status.value = await mailRepository.setMasterPassword(currentPassword, newPassword)
      return status.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to set master password'
      throw err
    } finally {
      isBusy.value = false
    }
  }

  const clearMasterPassword = async (currentPassword: string) => {
    isBusy.value = true
    error.value = null
    try {
      status.value = await mailRepository.clearMasterPassword(currentPassword)
      return status.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to clear master password'
      throw err
    } finally {
      isBusy.value = false
    }
  }

  const lockCurrentSession = async () => {
    isBusy.value = true
    error.value = null
    try {
      status.value = await mailRepository.lockCurrentSession()
      return status.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to lock current session'
      throw err
    } finally {
      isBusy.value = false
    }
  }

  return {
    status,
    isReady,
    isBusy,
    error,
    requiresUnlock,
    hasMissingStorageKey,
    hasKeyringIssue,
    initialize,
    refreshStatus,
    unlock,
    setMasterPassword,
    clearMasterPassword,
    lockCurrentSession,
  }
})
