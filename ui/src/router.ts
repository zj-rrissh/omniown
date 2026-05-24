import { createRouter, createWebHashHistory } from 'vue-router'

const SearchView = () => import('./views/SearchView.vue')
const DocumentsView = () => import('./views/DocumentsView.vue')
const ConfigView = () => import('./views/ConfigView.vue')
const StatusView = () => import('./views/StatusView.vue')

const routes = [
  { path: '/', name: 'search', component: SearchView },
  { path: '/documents', name: 'documents', component: DocumentsView },
  { path: '/config', name: 'config', component: ConfigView },
  { path: '/status', name: 'status', component: StatusView },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
