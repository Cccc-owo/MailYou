import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type {
  AccountSetupDraft,
  MailAccount,
  OAuthAuthorizationRequest,
  OAuthProviderAvailability,
} from '@/types/account'
import type { SyncStatus } from '@/types/mail'
import { mailRepository } from '@/services/mail'

const STORAGE_KEY = 'mailyou.currentAccountId'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<MailAccount[]>([])
  const currentAccountId = ref<string | null>(localStorage.getItem(STORAGE_KEY))
  const isLoading = ref(false)
  const isTestingConnection = ref(false)
  const oauthProviders = ref<OAuthProviderAvailability[]>([])
  const error = ref<string | null>(null)
  const connectionStatus = ref<SyncStatus | null>(null)

  const currentAccount = computed(() =>
    accounts.value.find((account) => account.id === currentAccountId.value) ?? null,
  )

  const isCurrentAccountPop3 = computed(() =>
    currentAccount.value?.incomingProtocol === 'pop3'
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

  const syncAccount = async (accountId: string) => {
    error.value = null

    try {
      connectionStatus.value = await mailRepository.syncAccount(accountId)
      return connectionStatus.value
    } catch (syncError) {
      error.value = syncError instanceof Error ? syncError.message : 'Unable to sync mailbox'
      throw syncError
    }
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

  const getAccountConfig = async (accountId: string) => {
    return await mailRepository.getAccountConfig(accountId)
  }

  const updateAccount = async (accountId: string, draft: AccountSetupDraft) => {
    const updated = await mailRepository.updateAccount(accountId, draft)
    const idx = accounts.value.findIndex((a) => a.id === accountId)
    if (idx !== -1) {
      accounts.value[idx] = updated
    }
    return updated
  }

  const loadOAuthProviders = async () => {
    oauthProviders.value = await mailRepository.listOAuthProviders()
    return oauthProviders.value
  }

  const authorizeOAuth = async (request: OAuthAuthorizationRequest) => {
    return await mailRepository.authorizeOAuth(request)
  }

  return {
    accounts,
    currentAccount,
    currentAccountId,
    isCurrentAccountPop3,
    isLoading,
    isTestingConnection,
    oauthProviders,
    error,
    connectionStatus,
    loadAccounts,
    createAccount,
    deleteAccount,
    getAccountConfig,
    updateAccount,
    loadOAuthProviders,
    authorizeOAuth,
    selectAccount,
    syncAccount,
    testAccountConnection,
  }
})
