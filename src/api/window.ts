import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

export async function setAlwaysOnTop(value: boolean): Promise<void> {
  await appWindow.setAlwaysOnTop(value)
}

export async function minimizeWindow(): Promise<void> {
  await appWindow.minimize()
}

export async function closeWindow(): Promise<void> {
  await appWindow.close()
}

export async function getAlwaysOnTop(): Promise<boolean> {
  return await appWindow.isAlwaysOnTop()
}
