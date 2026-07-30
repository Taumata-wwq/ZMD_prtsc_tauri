/** 区域名工具：处理 "分类-子名" 格式的区域名 */

/**
 * 拼接完整区域名
 * - category 为 "自定义" 时返回 "自定义"
 * - sub 为空字符串时返回 category
 */
export function buildRegionName(category: string, sub: string): string {
  if (category === '自定义') return '自定义'
  if (!sub) return category
  return `${category}-${sub}`
}

/** 去掉分类前缀（如 "四号谷地-枢纽区" → "枢纽区"）；不含 '-' 时返回原字符串 */
export function stripCategoryPrefix(name: string): string {
  const idx = name.indexOf('-')
  return idx > 0 ? name.substring(idx + 1) : name
}

/** 获取分类前缀（如 "四号谷地-枢纽区" → "四号谷地"）；不含 '-' 时返回 null */
export function getCategoryPrefix(name: string): string | null {
  const idx = name.indexOf('-')
  return idx > 0 ? name.substring(0, idx) : null
}
