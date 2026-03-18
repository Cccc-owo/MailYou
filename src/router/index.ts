import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'mail',
    component: () => import('@/views/MailShellView.vue'),
  },
  {
    path: '/account-setup/:accountId?',
    name: 'account-setup',
    component: () => import('@/views/AccountSetupView.vue'),
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/SettingsView.vue'),
  },
  {
    path: '/contacts',
    name: 'contacts',
    component: () => import('@/views/ContactsView.vue'),
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
