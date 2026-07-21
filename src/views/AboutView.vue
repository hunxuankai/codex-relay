<script setup lang="ts">
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
          在不同 Provider 和 API Key 之间切换。
        </p>
      </div>
    </header>

    <section class="info-card current-info" aria-labelledby="current-info-title">
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
      <button type="button" aria-label="打开当前 Codex 配置目录" @click="$emit('openDirectory')">
        打开配置目录
      </button>
    </section>

    <section class="info-card" aria-labelledby="workflow-title">
      <h2 id="workflow-title">工作原理</h2>
      <ol class="workflow-list">
        <li><code>config.toml</code> 保存 Provider 地址、模型等非秘密配置。</li>
        <li><code>providers.json</code> 保存每个 Provider 对应的 API Key。</li>
        <li>切换 Provider 时，将目标配置同步到 <code>config.toml</code>，并将目标密钥写入 <code>auth.json</code>。</li>
        <li>每次受管写入前创建备份；写入失败时尝试恢复所有已触及文件。</li>
      </ol>
    </section>

    <section class="info-card" aria-labelledby="modified-title">
      <h2 id="modified-title">会修改哪些内容</h2>
      <div class="content-grid">
        <article class="content-section">
          <h3>Codex 配置目录</h3>
          <ul>
            <li>
              <code>config.toml</code>：新增、编辑或删除 <code>model_providers</code> 中的目标
              Provider。启用或切换时还会更新顶层 <code>model_provider</code> 和
              <code>cli_auth_credentials_store</code>；若目标 Provider 配置了默认模型，还会更新
              顶层 <code>model</code>，否则保留原值。
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
            <li><code>providers.json</code>：各 Provider 的 API Key。</li>
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
          </ul>
        </article>
      </div>
    </section>

    <section class="info-card warning-card" aria-labelledby="security-title">
      <h2 id="security-title">数据与安全</h2>
      <ul>
        <li><code>providers.json</code>、<code>auth.json</code> 和配置备份可能包含明文 API Key。</li>
        <li>本程序不会调用模型接口验证 Base URL 或 API Key；自动更新检查失败时静默处理，也不会自动下载或安装。</li>
        <li>从界面清空密钥只会修改本机文件，不会在 Provider 平台吊销远端凭据。</li>
        <li>卸载程序不会删除 Codex 配置、Codex Relay 应用数据、API Key、日志或备份。</li>
      </ul>
    </section>
  </main>
</template>

<style scoped>
.about-view,
.info-card,
.content-grid,
.content-section,
.workflow-list,
.info-list {
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

.info-card {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  padding: 1rem;
  background: var(--surface);
}

.current-info {
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
.workflow-list {
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
