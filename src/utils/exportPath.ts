/**
 * 截图导出路径构建工具
 * 文件名冲突由 Rust 端 resolve_unique_path 处理，本模块只负责拼装字符串。
 */

/**
 * 展开文件名模式中的 {key} 占位符
 * 未在 vars 中出现的占位符保持原样（保留字面量）
 */
export function expandFilenamePattern(
  pattern: string,
  vars: Record<string, string>,
): string {
  return pattern.replace(/\{(\w+)\}/g, (match, key: string) => {
    return Object.prototype.hasOwnProperty.call(vars, key) ? vars[key] : match
  })
}

/**
 * 构建完整保存路径
 * @param dir 目录路径（末尾可带可不带路径分隔符）
 * @param pattern 文件名模式
 * @param vars 占位符键值对
 * @param ext 扩展名（不带点，如 "jpg" 或 "JPG"），内部统一转小写
 * @param suffix 追加到展开后文件名末尾的后缀（如 "_crop"），默认空
 */
export function buildSavePath(
  dir: string,
  pattern: string,
  vars: Record<string, string>,
  ext: string,
  suffix = '',
): string {
  let stem = expandFilenamePattern(pattern, vars)
  if (suffix) stem += suffix
  const safeExt = ext.toLowerCase()
  const sep = dir.endsWith('/') || dir.endsWith('\\') ? '' : '/'
  return `${dir}${sep}${stem}.${safeExt}`
}
