import { createRouter, createWebHashHistory } from 'vue-router'
import ConfigView from './views/ConfigView.vue'
import DocumentsView from './views/DocumentsView.vue'
import SearchView from './views/SearchView.vue'
import StatusView from './views/StatusView.vue'

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
