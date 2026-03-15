import { createRouter, createWebHashHistory } from 'vue-router'
import MailShellView from '@/views/MailShellView.vue'
import AccountSetupView from '@/views/AccountSetupView.vue'
import SettingsView from '@/views/SettingsView.vue'
import ContactsView from '@/views/ContactsView.vue'

const routes = [
  {
    path: '/',
    name: 'mail',
    component: MailShellView,
  },
  {
    path: '/account-setup/:accountId?',
    name: 'account-setup',
    component: AccountSetupView,
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView,
  },
  {
    path: '/contacts',
    name: 'contacts',
    component: ContactsView,
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
