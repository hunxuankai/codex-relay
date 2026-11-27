import { computed, onScopeDispose, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { CliProfileList, ProfileKeyEdit, ProfileKeySaved } from '../types/cliProfile'

function messageFor(error: unknown, fallback: string) {
  return error instanceof relay.RelayCommandError ? error.message : fallback
}

export function useCliProfiles() {
  const inventory = shallowRef<CliProfileList | null>(null)
  const edit = shallowRef<ProfileKeyEdit | null>(null)
  const loading = shallowRef(false)
  const action = shallowRef<'prepare' | 'save' | null>(null)
  const readError = shallowRef<string | null>(null)
  const actionError = shallowRef<string | null>(null)
  const success = shallowRef<{ message: string; id: number } | null>(null)
  const busy = computed(() => loading.value || action.value !== null || edit.value !== null)
  const saving = computed(() => action.value === 'save')
  let active = true
  let messageId = 0

  onScopeDispose(() => {
    active = false
    edit.value = null
  })

  function showSuccess(message: string) {
    actionError.value = null
    success.value = { message, id: ++messageId }
  }

  function showError(error: unknown, fallback: string) {
    success.value = null
    actionError.value = messageFor(error, fallback)
  }

  async function refresh() {
    if (!active || busy.value) return
    loading.value = true
    readError.value = null
    try {
      const next = await relay.listCliProfiles()
      if (active) inventory.value = next
    } catch (error) {
      if (active) readError.value = messageFor(error, '无法读取 CLI Profiles。请刷新后重试。')
    } finally {
      if (active) loading.value = false
    }
  }

  async function prepare(profileName: string) {
    if (!active || busy.value || readError.value) return
    action.value = 'prepare'
    actionError.value = null
    success.value = null
    try {
      const next = await relay.prepareProfileKeyEdit(profileName)
      if (active) edit.value = next
    } catch (error) {
      if (active) showError(error, '无法打开 Key 设置。请刷新后重试。')
    } finally {
      if (active) action.value = null
    }
    if (!edit.value) await refresh()
  }

  function cancel() {
    if (!saving.value) edit.value = null
  }

  async function runSave(request: (target: ProfileKeyEdit) => Promise<ProfileKeySaved>) {
    const target = edit.value
    if (!active || !target || saving.value) return
    action.value = 'save'
    try {
      const outcome = await request(target)
      if (active) {
        showSuccess(outcome.windowsNotified
          ? '已保存到 Windows 用户环境变量。请从已刷新的终端环境重新启动 Codex。'
          : '已保存到 Windows 用户环境变量，但未确认系统刷新通知。请完全退出终端后重新打开。')
      }
    } catch (error) {
      if (active) showError(error, '无法确认保存结果。请刷新后核对 Windows 用户变量。')
    } finally {
      if (active) {
        edit.value = null
        action.value = null
      }
    }
    await refresh()
  }

  function save(newKey: string) {
    return runSave(({ profileName, token }) => {
      const pending = relay.saveProfileEnvKey({ profileName, token, newKey })
      newKey = ''
      return pending
    })
  }

  function applyProviderKey(apiKeyId: string) {
    return runSave(({ profileName, token }) => relay.applyProfileProviderKey({ profileName, token, apiKeyId }))
  }

  async function copy(value: string, kind: 'command' | 'variable') {
    if (!active) return
    try {
      await navigator.clipboard.writeText(value)
      if (active) showSuccess(kind === 'command' ? '启动命令已复制。' : '变量名已复制。')
    } catch (error) {
      if (active) showError(error, '无法访问剪贴板。可直接选中页面上的文字复制。')
    }
  }

  async function openWindowsSettings() {
    try {
      await relay.openProfileEnvironmentSettings()
    } catch (error) {
      if (active) showError(error, '可在 Windows 开始菜单搜索「编辑帐户的环境变量」。')
    }
  }

  return {
    inventory: readonly(inventory),
    edit: readonly(edit),
    loading: readonly(loading),
    busy,
    saving,
    readError: readonly(readError),
    actionError: readonly(actionError),
    success: readonly(success),
    refresh,
    prepare,
    cancel,
    save,
    applyProviderKey,
    copy,
    openWindowsSettings,
  }
}
