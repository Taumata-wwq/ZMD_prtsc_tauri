/** 将 0-255 的数值转为两位十六进制字符串 */
export function toHex(n: number): string {
  return n.toString(16).padStart(2, '0')
}

/**
 * 调整颜色亮度/透明度
 * @param hex 6 位十六进制颜色（如 "#ff0000" 或 "ff0000"），非 6 位则原样返回
 * @param lightnessPercent 亮度调整百分比：正数变亮（向 255 推进），负数变暗（向 0 缩减）
 * @param alpha 透明度（0-1），传入则返回 rgba 格式；不传则返回 #rrggbb 格式
 */
export function adjustColor(
  hex: string,
  lightnessPercent: number,
  alpha: number | null = null,
): string {
  const h = hex.replace('#', '')
  if (h.length !== 6) return hex
  const r = parseInt(h.slice(0, 2), 16)
  const g = parseInt(h.slice(2, 4), 16)
  const b = parseInt(h.slice(4, 6), 16)
  const adj = (c: number) => {
    const v = lightnessPercent >= 0
      ? c + (255 - c) * lightnessPercent / 100
      : c * (1 + lightnessPercent / 100)
    return Math.max(0, Math.min(255, Math.round(v)))
  }
  const rr = adj(r), gg = adj(g), bb = adj(b)
  if (alpha !== null) {
    return `rgba(${rr}, ${gg}, ${bb}, ${alpha})`
  }
  return `#${toHex(rr)}${toHex(gg)}${toHex(bb)}`
}
