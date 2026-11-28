<script setup lang="ts">
import { ElButton, ElCard, ElTag } from 'element-plus'
import type { CliProfile, KeyRestriction, ProfileProblem, VariablePresence } from '../types/cliProfile'

defineProps<{
  profiles: readonly CliProfile[]
  editBlocked: boolean
}>()

const emit = defineEmits<{
  copy: [text: string, kind: 'command' | 'variable']
  edit: [name: string]
}>()

const presence: Record<VariablePresence, string> = {
  set: '已设置',
  notDetected: '未检测到',
  unavailable: '无法检测',
}

const problemText: Record<ProfileProblem, string> = {
  invalidConfiguration: '配置无法解析，或 Profile 文件名不受支持。',
  missingProvider: '无法确定此 Profile 的自定义供应商。',
  missingEnvironmentVariable: '请在对应供应商的配置中设置有效的 env_key。',
  unsupportedAuthentication: '此认证方式或供应商地址不支持在 Relay 中管理 Key。',
}

const restrictionText: Record<KeyRestriction, string> = {
  sharedVariable: '共享变量：',
  systemVariable: '存在同名的 Windows 系统变量，请在 Windows 中管理。',
  notDedicated: '不是受支持的专用变量。请使用独立的 *_API_KEY 或 *_TOKEN。',
  unsupportedValue: '用户变量不是受支持的普通字符串，请在 Windows 中管理。',
  environmentUnavailable: '当前无法读取或管理 Windows 用户变量。',
}
</script>

<template>
  <ul class="profile-list" aria-label="CLI Profile 列表">
    <li v-for="(profile, index) in profiles" :key="profile.command ?? `${profile.name}:${index}`">
      <ElCard class="profile-card" shadow="never">
        <header class="profile-card-header">
          <div class="profile-identity">
            <h2>{{ profile.name || '未命名 Profile' }}</h2>
            <ElTag v-if="profile.inheritsDefault" type="info" effect="plain" size="small">继承默认配置</ElTag>
            <span v-if="profile.providerId" class="profile-route">
              {{ profile.providerId }}<template v-if="profile.host"> · {{ profile.host }}</template>
            </span>
          </div>
          <code v-if="profile.command" class="profile-command">{{ profile.command }}</code>
        </header>

        <p v-if="profile.problem" class="profile-note">{{ problemText[profile.problem] }}</p>
        <template v-else-if="profile.variable && profile.keyStatus">
          <div class="profile-credentials">
            <code>{{ profile.variable }}</code>
            <ElTag :type="profile.keyStatus.user === 'set' ? 'success' : 'info'" effect="plain" size="small">
              Windows 用户变量：{{ presence[profile.keyStatus.user] }}
            </ElTag>
            <span v-if="profile.keyStatus.user === 'notDetected' && profile.keyStatus.relayProcess === 'set' && profile.keyStatus.restriction !== 'systemVariable'">
              仅当前 Relay 进程可见
            </span>
            <span v-else>
              Relay 进程：{{ profile.keyStatus.relayProcess === 'set' ? '可见' : presence[profile.keyStatus.relayProcess] }}
            </span>
          </div>
          <p v-if="profile.keyStatus.user === 'set'" class="profile-note">
            当前 Key（Windows 用户变量）：{{ profile.keyStatus.matchedUserKeyName ?? '未能匹配已保存的 Key' }}
          </p>
          <p v-if="profile.keyStatus.restriction" class="profile-note">
            {{ restrictionText[profile.keyStatus.restriction] }}<template v-if="profile.keyStatus.restriction === 'sharedVariable'">{{ profile.keyStatus.sharedWith.join('、') }}。请为各 Profile 使用专用变量。</template>
          </p>
        </template>

        <div v-if="profile.command || profile.keyStatus" class="profile-actions">
          <ElButton
            v-if="profile.command"
            native-type="button"
            type="primary"
            plain
            size="small"
            :aria-label="`复制 ${profile.name} 的启动命令`"
            @click="emit('copy', profile.command, 'command')"
          >复制启动命令</ElButton>
          <template v-if="profile.variable && profile.keyStatus">
            <ElButton
              v-if="profile.keyStatus.restriction"
              native-type="button"
              size="small"
              :aria-label="`复制 ${profile.name} 的变量名`"
              @click="emit('copy', profile.variable, 'variable')"
            >复制变量名</ElButton>
            <ElButton
              v-else
              native-type="button"
              size="small"
              :aria-disabled="editBlocked"
              :aria-label="`${profile.keyStatus.user === 'set' ? '更换' : '设置'} ${profile.name} 的 Key`"
              @click="!editBlocked && emit('edit', profile.name)"
            >{{ profile.keyStatus.user === 'set' ? '更换 Key' : '设置 Key' }}</ElButton>
          </template>
        </div>
      </ElCard>
    </li>
  </ul>
</template>

<style scoped>
.profile-list {
  display: grid;
  gap: 0.65rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.profile-list > li {
  min-width: 0;
}

.profile-card :deep(.el-card__body) {
  display: grid;
  gap: 0.55rem;
  padding: 0.85rem 1rem;
}

.profile-card-header,
.profile-identity,
.profile-credentials,
.profile-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.35rem 0.65rem;
  min-width: 0;
  overflow-wrap: anywhere;
}

.profile-card-header {
  justify-content: space-between;
}

.profile-identity h2 {
  margin: 0;
  font-size: 1.1rem;
}

.profile-route,
.profile-note,
.profile-credentials {
  color: var(--text-secondary);
  font-size: 0.875rem;
}

.profile-credentials code,
.profile-command {
  color: var(--text-primary);
}

.profile-credentials,
.profile-actions {
  align-items: center;
}

.profile-note {
  margin: 0;
  overflow-wrap: anywhere;
}

.profile-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
</style>
