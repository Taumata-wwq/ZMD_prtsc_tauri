/** 将数值夹紧到 [min, max] 区间 */
export function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v))
}
