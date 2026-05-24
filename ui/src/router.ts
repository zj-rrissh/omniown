import { createRouter, createWebHashHistory } from 'vue-router'
import ConfigView from './views/ConfigView.vue'
import StatusView from './views/StatusView.vue'
import SearchView from './views/SearchView.vue'

const routes = [
  { path: '/', name: 'search', component: SearchView },
  { path: '/config', name: 'config', component: ConfigView },
  { path: '/status', name: 'status', component: StatusView },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
