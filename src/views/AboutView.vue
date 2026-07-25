<script setup lang="ts">
import { ElButton, ElCard } from 'element-plus'
defineProps<{
  appVersion: string | null
  configDirectory: string | null
}>()

defineEmits<{
  openDirectory: []
}>()
</script>

<template>
  <main class="about-view" aria-label="关于 Codex Relay">
    <header class="view-header">
      <div>
        <p class="eyebrow">About</p>
        <h1>关于 Codex Relay</h1>
        <p class="summary">
          Codex Relay 是一个本机 Provider 配置管理工具，通过受保护的文件事务让 Codex CLI
          在不同 Provider 之间切换，并为每个 Provider 独立切换多个命名 Base URL 与 API Key。
        </p>
      </div>
    </header>

    <ElCard class="info-card current-info" shadow="never" aria-labelledby="current-info-title">
      <h2 id="current-info-title">当前信息</h2>
      <dl class="info-list">
        <div>
          <dt>软件版本</dt>
          <dd>当前版本：{{ appVersion ?? '暂不可用' }}</dd>
        </div>
        <div>
          <dt>Codex 配置目录</dt>
          <dd><code>{{ configDirectory ?? '正在检测' }}</code></dd>
        </div>
      </dl>
      <ElButton type="primary" plain native-type="button" aria-label="打开当前 Codex 配置目录" @click="$emit('openDirectory')">
        打开配置目录
      </ElButton>
    </ElCard>

    <ElCard class="info-card" shadow="never" aria-labelledby="workflow-title">
      <h2 id="workflow-title">工作原理</h2>
      <ol class="workflow-list">
        <li><code>config.toml</code> 保存每个 Provider 当前实际 Base URL，以及 Codex 顶层当前模型、推理强度等官方配置。</li>
        <li><code>provider-preferences.json</code> 保存多个命名 Base URL、模型集合和逐模型推理强度偏好。</li>
        <li><code>providers.json</code> 保存每个 Provider 的多个命名 API Key 与密钥预选。</li>
        <li>Base URL 与 API Key 可以独立切换；当前 Provider 立即同步，非当前 Provider 只保存预选。</li>
        <li>切换 Provider 时，将其预选地址、密钥、模型和推理强度同步到 <code>config.toml</code> 与 <code>auth.json</code>。</li>
        <li>每次受管写入前创建备份；写入失败时尝试恢复所有已触及文件。</li>
      </ol>
    </ElCard>

    <ElCard class="info-card" shadow="never" aria-labelledby="availability-title">
      <h2 id="availability-title">Provider 可用性测试</h2>
      <p class="section-intro">
        应用启动、自检、Provider 列表刷新和文件监控不会访问 Provider 模型网络；只有用户显式点击测试时才访问 Provider 模型网络。
      </p>
      <ul class="availability-list">
        <li>
          <strong>API 可用性测试</strong>：发送一次无工具、非流式、最多 16 个输出 token 的最小 Responses 请求，
          用于确认地址、认证、模型和响应格式，通常只产生少量 token 费用。
        </li>
        <li>
          <strong>Codex 兼容性测试</strong>：启动本机 Codex 并发送一次正常 Codex 回合，
          会比 API 测试消耗更多 token 并等待更久；测试不会修改当前 config.toml 或 auth.json。
        </li>
        <li>测试结果只保存在本次会话内；Provider 配置发生变化后，旧结果会失效，也不会写入日志或应用数据。</li>
      </ul>
    </ElCard>

    <ElCard class="info-card" shadow="never" aria-labelledby="modified-title">
      <h2 id="modified-title">会修改哪些内容</h2>
      <div class="content-grid">
        <article class="content-section">
          <h3>Codex 配置目录</h3>
          <ul>
            <li>
              <code>config.toml</code>：新增、编辑或删除 <code>model_providers</code> 中的目标
              Provider。启用或切换时还会更新顶层 <code>model_provider</code>、<code>model</code>、
              <code>model_reasoning_effort</code> 和 <code>cli_auth_credentials_store</code>。Relay
              不会把私有偏好写入 <code>[model_providers.&lt;id&gt;]</code>。
            </li>
            <li>
              <code>auth.json</code>：启用或切换时重新生成，只保存当前生效的
              <code>OPENAI_API_KEY</code>。
            </li>
          </ul>
          <p>
            <code>config.toml</code> 使用局部 TOML 编辑保留注释、未知字段、其他 Provider、MCP、
            features、sandbox 和 profiles；落盘时仍会原子替换整个文件。
          </p>
        </article>

        <article class="content-section">
          <h3>Codex Relay 应用数据</h3>
          <ul>
            <li><code>providers.json</code>：各 Provider 的多个命名 API Key 和密钥预选。</li>
            <li><code>provider-preferences.json</code>：各 Provider 的多个命名 Base URL、可用模型、当前偏好和逐模型推理强度。</li>
            <li><code>settings.json</code>：窗口、托盘、首次引导、自启动和应用网络代理设置。</li>
            <li>
              <code>backups/</code>：配置事务快照、元数据和设置备份；备份页可展开事务文件列表，
              并使用 Windows 记事本打开所选文件。
            </li>
            <li><code>logs/</code>：经过密钥脱敏的软件日志。</li>
            <li><code>transaction.json</code>：配置事务进行期间使用的临时标记。</li>
          </ul>
        </article>

        <article class="content-section">
          <h3>Windows 系统状态</h3>
          <ul>
            <li>只有用户启用或关闭“开机启动”时，才会修改 Windows 自启动状态。</li>
            <li>应用会在启动时及运行期间每小时自动检查一次更新，但只有用户明确确认后才会下载并安装。</li>
            <li>已安装版本升级会沿用原安装目录；如果要更换安装位置，请先卸载旧版再重新安装。</li>
          </ul>
        </article>
      </div>
    </ElCard>

    <ElCard class="info-card warning-card" shadow="never" aria-labelledby="security-title">
      <h2 id="security-title">数据与安全</h2>
      <ul>
        <li><code>providers.json</code>、<code>auth.json</code> 和配置备份可能包含明文 API Key。</li>
        <li>API Key 只在用户打开“管理与查看”对话框时返回前端，打开管理器后默认明文显示；关闭后清空该对话框的密钥状态。</li>
        <li>除用户显式启动 Provider 可用性或 Codex 兼容性测试外，本程序不会调用模型接口验证 Base URL 或 API Key；自动更新检查失败时静默处理，也不会自动下载或安装。</li>
        <li>删除或替换本机命名密钥不会在 Provider 平台吊销远端凭据；怀疑泄漏时必须在 Provider 平台轮换。</li>
        <li>卸载程序不会删除 Codex 配置、Codex Relay 应用数据、API Key、日志或备份。</li>
      </ul>
    </ElCard>
  </main>
</template>

<style scoped>
.about-view,
.content-grid,
.content-section,
.workflow-list,
.info-list,
.availability-list {
  display: grid;
  gap: 1rem;
}

.about-view {
  padding: 1.25rem;
}

.view-header h1,
.eyebrow,
.summary,
.info-card h2,
.content-section h3,
.content-section p,
.section-intro,
.info-list,
.info-list dd {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.summary {
  max-width: 52rem;
  color: var(--text-secondary);
  line-height: 1.65;
}

.section-intro {
  color: var(--text-secondary);
  line-height: 1.65;
}

.info-card {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  background: var(--surface);
}

.info-card :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
  padding: 1rem;
}

.current-info :deep(.el-card__body) {
  justify-items: start;
}

.info-list {
  width: 100%;
}

.info-list div {
  display: grid;
  grid-template-columns: minmax(8rem, 12rem) minmax(0, 1fr);
  gap: 1rem;
}

.info-list dt {
  color: var(--text-secondary);
  font-weight: 700;
}

.info-list dd,
.content-section li,
.workflow-list li {
  line-height: 1.65;
}

.content-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.content-section {
  align-content: start;
  border-radius: 0.65rem;
  padding: 1rem;
  background: var(--surface-muted);
}

.content-section:last-child {
  grid-column: 1 / -1;
}

.content-section ul,
.warning-card ul,
.workflow-list,
.availability-list {
  margin: 0;
  padding-left: 1.3rem;
}

.warning-card {
  border-color: var(--warning-border);
  background: var(--warning-soft);
}

code {
  overflow-wrap: anywhere;
}

@media (max-width: 720px) {
  .content-grid {
    grid-template-columns: 1fr;
  }

  .content-section:last-child {
    grid-column: auto;
  }

  .info-list div {
    grid-template-columns: 1fr;
    gap: 0.25rem;
  }
}
</style>
