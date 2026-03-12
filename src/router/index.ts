import { createRouter, createWebHashHistory } from 'vue-router'
import MailShellView from '@/views/MailShellView.vue'
import AccountSetupView from '@/views/AccountSetupView.vue'
import SettingsView from '@/views/SettingsView.vue'

const routes = [
  {
    path: '/',
    name: 'mail',
    component: MailShellView,
  },
  {
    path: '/account-setup',
    name: 'account-setup',
    component: AccountSetupView,
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView,
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
