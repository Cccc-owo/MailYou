import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { AccountSetupDraft, MailAccount } from '@/types/account'
import { mailRepository } from '@/services/mail'

const STORAGE_KEY = 'mailstack.currentAccountId'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<MailAccount[]>([])
  const currentAccountId = ref<string | null>(localStorage.getItem(STORAGE_KEY))
  const isLoading = ref(false)
  const error = ref<string | null>(null)

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

  const createAccount = async (draft: AccountSetupDraft) => {
    const account = await mailRepository.createAccount(draft)
    accounts.value = [account, ...accounts.value]
    selectAccount(account.id)
    return account
  }

  return {
    accounts,
    currentAccount,
    currentAccountId,
    isLoading,
    error,
    loadAccounts,
    createAccount,
    selectAccount,
  }
})
