<script setup lang="ts">
import { onMounted } from 'vue'
import { ElAlert, ElButton, ElEmpty } from 'element-plus'
import AppNotification from '../components/AppNotification.vue'
import CliProfileList from '../components/CliProfileList.vue'
import ProfileKeyDialog from '../components/ProfileKeyDialog.vue'
import { useCliProfiles } from '../composables/useCliProfiles'

const {
  inventory, edit, loading, busy, saving, readError, actionError, success,
  refresh, prepare, cancel, save, applyProviderKey, copy, openWindowsSettings,
} = useCliProfiles()

onMounted(refresh)
</script>

<template>
  <main class="cli-profiles-view" aria-label="CLI Profiles">
    <header class="profiles-page-header">
      <div>
        <h1>CLI Profiles</h1>
        <p>管理已有 Profile 的环境变量 Key，继续用原来的命令启动 Codex。</p>
      </div>
      <div class="profiles-page-actions">
        <ElButton native-type="button" aria-label="打开 Windows 环境变量" @click="openWindowsSettings">
          Windows 环境变量
        </ElButton>
        <ElButton native-type="button" :loading="loading" :disabled="busy" @click="refresh">刷新</ElButton>
      </div>
    </header>
    <div v-if="inventory" class="profile-location">
      <span>扫描目录</span><code>{{ inventory.codexHome }}</code>
      <span>终端应使用同一 <code>CODEX_HOME</code>，独立文件适用于 Codex CLI 0.134.0 及以上。</span>
    </div>
    <p class="profile-help">仅解析用户配置。现有终端和 Codex 不会更新，脚本或会话变量仍可覆盖；读到旧值时，请完全退出终端后重新打开。</p>

    <AppNotification :message="success?.message ?? null" :message-id="success?.id" level="success" />
    <AppNotification :message="actionError" level="error" />
    <AppNotification :message="readError" level="error" />
    <ElAlert
      v-if="inventory?.pendingWrite"
      title="有一次保存结果未确认，请核对 Windows 用户变量。Relay 不会自动还原旧值。"
      type="warning"
      :closable="false"
      show-icon
    />

    <section aria-label="Profile 扫描结果" :aria-busy="loading">
      <CliProfileList
        v-if="inventory?.profiles.length"
        :profiles="inventory.profiles"
        :edit-blocked="busy || Boolean(readError)"
        @copy="copy"
        @edit="prepare"
      />
      <p v-else-if="loading" role="status">正在扫描配置目录…</p>
      <ElEmpty v-else-if="inventory && !readError" description="未找到独立的 *.config.toml Profile 文件。" :image-size="72" />
    </section>

    <ProfileKeyDialog
      :edit="edit"
      :saving="saving"
      :save="save"
      :apply-provider-key="applyProviderKey"
      @cancel="cancel"
    />
  </main>
</template>

<style scoped>
.cli-profiles-view {
  display: grid;
  align-content: start;
  min-width: 0;
  min-height: 0;
  gap: 0.75rem;
  padding: 1rem;
  overflow-y: auto;
}

.profiles-page-header,
.profiles-page-actions,
.profile-location {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem 0.65rem;
}

.profiles-page-header {
  justify-content: space-between;
}

.profiles-page-header h1,
.profiles-page-header p,
.profile-help {
  margin: 0;
}

.profiles-page-header h1 {
  font-size: 1.5rem;
}

.profiles-page-header p,
.profile-location,
.profile-help {
  color: var(--text-secondary);
  font-size: 0.875rem;
  overflow-wrap: anywhere;
}

.profiles-page-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.profile-location {
  border: 1px solid var(--border);
  border-radius: 0.6rem;
  padding: 0.45rem 0.7rem;
  background: var(--surface-muted);
}
</style>
