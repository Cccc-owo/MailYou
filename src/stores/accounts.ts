import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { AccountSetupDraft, MailAccount } from '@/types/account'
import type { SyncStatus } from '@/types/mail'
import { mailRepository } from '@/services/mail'

const STORAGE_KEY = 'mailstack.currentAccountId'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<MailAccount[]>([])
  const currentAccountId = ref<string | null>(localStorage.getItem(STORAGE_KEY))
  const isLoading = ref(false)
  const isTestingConnection = ref(false)
  const error = ref<string | null>(null)
  const connectionStatus = ref<SyncStatus | null>(null)

  const currentAccount = computed(() =>
    accounts.value.find((account) => account.id === currentAccountId.value) ?? null,
  )

  const loadAccounts = async () => {
    isLoading.value = true
    error.value = null

    try {
      accounts.value = await mailRepository.listAccounts()

      if (!currentAccountId.value && accounts.value.length > 0) {
        currentAccountId.value = accounts.value[0].id
      }

      if (
        currentAccountId.value &&
        !accounts.value.some((account) => account.id === currentAccountId.value)
      ) {
        currentAccountId.value = accounts.value[0]?.id ?? null
      }

      if (currentAccountId.value) {
        localStorage.setItem(STORAGE_KEY, currentAccountId.value)
      }
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load accounts'
    } finally {
      isLoading.value = false
    }
  }

  const selectAccount = (accountId: string) => {
    currentAccountId.value = accountId
    localStorage.setItem(STORAGE_KEY, accountId)
  }

  const testAccountConnection = async (draft: AccountSetupDraft) => {
    isTestingConnection.value = true
    connectionStatus.value = null
    error.value = null

    try {
      connectionStatus.value = await mailRepository.testAccountConnection(draft)
      return connectionStatus.value
    } catch (testError) {
      error.value = testError instanceof Error ? testError.message : 'Unable to connect to mailbox'
      throw testError
    } finally {
      isTestingConnection.value = false
    }
  }

  const createAccount = async (draft: AccountSetupDraft) => {
    const account = await mailRepository.createAccount(draft)
    accounts.value = [account, ...accounts.value]
    selectAccount(account.id)
    return account
  }

  const deleteAccount = async (accountId: string) => {
    await mailRepository.deleteAccount(accountId)
    accounts.value = accounts.value.filter((a) => a.id !== accountId)

    if (currentAccountId.value === accountId) {
      const next = accounts.value[0]?.id ?? null
      currentAccountId.value = next

      if (next) {
        localStorage.setItem(STORAGE_KEY, next)
      } else {
        localStorage.removeItem(STORAGE_KEY)
      }
    }
  }

  return {
    accounts,
    currentAccount,
    currentAccountId,
    isLoading,
    isTestingConnection,
    error,
    connectionStatus,
    loadAccounts,
    createAccount,
    deleteAccount,
    selectAccount,
    testAccountConnection,
  }
})
