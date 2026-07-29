import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './assets/styles/global.css'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')

// 禁用 webview 默认右键菜单（在所有环境中阻止 contextmenu 事件）
window.addEventListener('contextmenu', (e) => {
  e.preventDefault()
})

