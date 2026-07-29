import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

/** 设置窗口是否始终置顶 */
export async function setAlwaysOnTop(value: boolean): Promise<void> {
  await appWindow.setAlwaysOnTop(value)
}

/** 最小化窗口 */
export async function minimizeWindow(): Promise<void> {
  await appWindow.minimize()
}

/** 关闭窗口 */
export async function closeWindow(): Promise<void> {
  await appWindow.close()
}

/** 读取窗口当前是否处于置顶状态 */
export async function getAlwaysOnTop(): Promise<boolean> {
  return await appWindow.isAlwaysOnTop()
}

export { appWindow }
