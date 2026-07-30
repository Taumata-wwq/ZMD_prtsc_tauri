import { computed } from 'vue'
import { useSettingsStore } from '@/stores/settings.store'
import { zh } from '@/i18n/locales/zh'
import { en } from '@/i18n/locales/en'

export type Locale = 'zh' | 'en'
export type Dictionary = Record<string, string>

const DICTS: Record<Locale, Dictionary> = {
  zh,
  en,
}

/** 查找 key，缺失时回退到中文，再回退到 key 本身 */
function translate(locale: Locale, key: string): string {
  const dict = DICTS[locale] ?? DICTS.zh
  return dict[key] ?? DICTS.zh[key] ?? key
}

export function useI18n() {
  const settingsStore = useSettingsStore()
  const locale = computed<Locale>(() => settingsStore.settings?.language ?? 'zh')

  // t 为普通函数：内部访问 locale.value 自动建立响应式依赖，
  // 在模板/computed/watch 中调用时随 locale 变化自动更新；params 替换 {key} 占位符
  const t = (key: string, params?: Record<string, string | number>): string => {
    let str = translate(locale.value, key)
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
      }
    }
    return str
  }

  return { t, locale }
}
