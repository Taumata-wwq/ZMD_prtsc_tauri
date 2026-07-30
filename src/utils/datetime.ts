export function formatTimestamp(
  date: Date,
  pattern: 'YYYYMMDD_HHMMSS' | 'YYYY-MM-DD HH:mm:ss',
): string {
  const pad = (n: number) => n.toString().padStart(2, '0')
  const y = date.getFullYear()
  const mo = pad(date.getMonth() + 1)
  const d = pad(date.getDate())
  const h = pad(date.getHours())
  const mi = pad(date.getMinutes())
  const s = pad(date.getSeconds())
  if (pattern === 'YYYYMMDD_HHMMSS') {
    return `${y}${mo}${d}_${h}${mi}${s}`
  }
  return `${y}-${mo}-${d} ${h}:${mi}:${s}`
}

/** 返回本地化的日期时间字符串（24 小时制） */
export function formatDateTime(date: Date, locale: 'zh' | 'en'): string {
  const localeStr = locale === 'zh' ? 'zh-CN' : 'en-US'
  return date.toLocaleString(localeStr, { hour12: false })
}
