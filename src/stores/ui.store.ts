import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ViewKey = 'capture' | 'history' | 'data' | 'settings'

export const useUiStore = defineStore('ui', () => {
  const currentView = ref<ViewKey>('capture')

  function setView(view: ViewKey) {
    currentView.value = view
  }

  return {
    currentView,
    setView,
  }
})
