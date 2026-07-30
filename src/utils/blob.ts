/** 释放 Blob URL（仅 blob: 协议），避免内存泄漏 */
export function revokeBlobUrl(url: string | null | undefined): void {
  if (url && url.startsWith('blob:')) {
    URL.revokeObjectURL(url)
  }
}
