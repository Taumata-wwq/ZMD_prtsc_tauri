import { ref } from 'vue'
import type { Ref } from 'vue'

/**
 * 自动清空的错误信息 composable
 *
 * 用于全局错误条展示：setError 后 timeout 毫秒自动清空。
 * 重复调用 setError 会重置计时器。
 */
export function useAutoClearError(timeout = 3000): {
  error: Ref<string | null>
  setError: (msg: string) => void
} {
  const error = ref<string | null>(null)
  let timer: ReturnType<typeof setTimeout> | null = null

  function setError(msg: string) {
    error.value = msg
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      error.value = null
      timer = null
    }, timeout)
  }

  return { error, setError }
}
