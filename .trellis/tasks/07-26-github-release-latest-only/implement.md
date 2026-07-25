# GitHub Releases 仅保留最新版本下载实施计划

## 实施清单

1. [x] 更新 `src/release-retention.test.ts`，先写出清理工作流缺失时必然失败的结构
   断言（RED）。
2. [x] 新增 `.github/workflows/cleanup-old-releases.yml`：Release 正式发布后触发，
   校验 `releases/latest`，分页获取历史 Release，并逐项删除 Release 与对应 tag；
   提供安全的手动重试入口（GREEN）。
3. [x] 更新 README 与 `.trellis/spec/release/{index,updater,publishing}.md`，记录
   Latest-only 策略、历史链接失效范围、清理失败处置和远端证据边界。
4. [x] 重构 workflow 脚本和测试断言，确认当前 Release 排除、Draft/Prerelease 边界、
   分页和错误退出路径清晰可审计。
5. [ ] 运行专项测试、`git diff --check`，再运行完整 `npm run check`（若完整检查受
   环境限制失败，保留首次失败与根因）。
6. [ ] 检查 Git 跟踪/忽略范围和秘密扫描；确认没有真实 Token、Authorization、签名
   私钥、用户数据或真实 Codex/Relay 数据目录进入改动。
7. [ ] 仅在用户确认的彻底策略下，对当前公开仓库执行一次远端历史 Release/tag
   清理；记录真实命令输出和未完成项，不把本地测试当作远端成功证据。

## 行为切片与验证命令

### 切片 A：发布后只保留当前 Latest

- 输入/操作：触发 Release `published` 事件。
- 预期结果：清理工作流保留 `releases/latest` 的 ID/tag，历史 Release、资产和
  对应 tag 被删除；Draft 阶段不会触发该清理。
- 测试边界：结构测试读取 workflow 文本，不 mock 或调用真实 GitHub API。

### 切片 B：一致性或删除失败可见

- 输入/操作：Latest 与事件 Release 不一致，或任一删除 API 返回失败。
- 预期结果：脚本返回非零，Actions 标记失败，不输出“清理成功”；可通过手动入口
  重试。
- 测试边界：断言脚本包含显式校验和 `set -euo pipefail`/非零分支；真实网络失败在
  远端运行记录中单独保留。

### 切片 C：客户端更新入口不变

- 输入/操作：读取 Tauri updater endpoint 和发布 workflow。
- 预期结果：固定 `releases/latest/download/latest.json`、Draft、签名 Secret 和
  `releaseBody` 契约仍存在。
- 测试命令：

  ```powershell
  npx vitest run src/release-config.test.ts src/release-retention.test.ts
  ```

## 全量验证命令

```powershell
git diff --check
npx vitest run src/release-config.test.ts src/release-retention.test.ts
npm run check
git status --short --ignored
git ls-files | Select-String -Pattern '(^|/)(auth\.json|providers\.json|.*\.sig)$'
```

远端只读核对（清理前后分别执行）：

```powershell
gh release list --repo hunxuankai/codex-relay --limit 100
gh api --paginate repos/hunxuankai/codex-relay/tags --jq '.[].name'
```

## 风险文件与回滚点

- `.github/workflows/cleanup-old-releases.yml`：误删风险最高；修改后必须先通过结构
  测试，且保留 ID/tag 双重排除和一致性校验。
- `src/release-config.test.ts`、`src/release-retention.test.ts`：测试契约；若现行
  工作流确实改变，优先同步长期契约，不放宽断言掩盖缺失行为。
- README 与 release spec：只记录实际实现和证据边界，避免承诺可恢复历史下载。
- 远端 Release/tag：删除不可逆；执行前输出当前列表并确认只保留 `releases/latest`，
  失败后只允许重试存在性检查，不使用通配符清理。

## 进度与证据

- 规划阶段已确认：当前公开 Release 为 `v0.1.0` 至 `v0.2.1`，每个包含 3 项资产；
  `v0.2.1` 是 Latest；用户已选择同时删除历史 Git tags。
- RED：2026-07-26 运行 `npx vitest run src/release-retention.test.ts`，3 项全部按预期
  失败；失败原因是清理 workflow 尚不存在（空文本），未触发无关错误。
- GREEN：新增 workflow 后同一专项测试 3/3 通过；随后
  `npx vitest run src/release-config.test.ts src/release-retention.test.ts` 2 个文件、
  16 项通过。
- 结构检查：workflow 的 `bash -n` 通过，PyYAML 解析通过，`git diff --check` 通过
  （仅有 Git 的 LF/CRLF 提示）；完整检查待执行。
- 远端首次手动 Run `30177039434` 于 `2026-07-25T22:09:53Z` 失败且未删除资源。日志显示
  API 返回的 `draft=false`、`prerelease=false` 被 `.draft // true` / `.prerelease // true`
  误判为 `true`；新增布尔值回归测试先复现 1 项失败，随后改用 `| tostring`，专项 4/4、
  Bash 语法、Actionlint 和公开 Latest 字段模拟均通过。该次失败是实现缺陷，不能记为
  远端清理成功。
- 真实 GitHub Actions 和远端删除：待工作流进入默认分支后执行；未执行前不得声称
  历史资源已清理。
