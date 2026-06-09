# 更新日志

所有重要变更记录在此文件。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

---

## [1.1.1] - 2026-06-09

`v1.1.0` 热修复版本。

### 修复

- 修复保存后的文本记录点击“复制”时提示复制失败的问题。
- 修复前端 `copy_to_clipboard` 调用参数名与 Tauri 后端命令参数名不一致的问题。
- 剪贴板写入增加短暂重试,降低 Windows 剪贴板被其它程序短时占用时的失败率。
- 托盘菜单复制最近/置顶内容改为复用统一剪贴板写入逻辑。

### 验证

- `npm run build`
- `cargo test --manifest-path src-tauri\Cargo.toml`

## [1.1.0] - 2026-06-09

稳定版截图框选 OCR。此版本替代 2026-06-02 已撤回的旧 v1.1.0 方案。

### 新增

- 新增 `Ctrl+Shift+O` 框选屏幕 OCR 快捷键。
- 新增透明全屏 `screen_ocr` 窗口,拖拽选择屏幕区域后只截取选区识别。
- 新增 `capture_screen_ocr_region` Tauri 命令,负责隐藏框选层、截屏、调用 OCR 并返回文本。
- 新增 `set_pending_capture_text` / `take_pending_capture_text`,OCR 结果直接进入保存窗口。
- 新增 `src-tauri/src/ocr/wechat.rs` 作为 WeChatOCR 备用引擎封装。
- 打包时自动同步 `scripts/ocr_host` 产物到 `src-tauri/resources/ocr_host`。

### 变更

- 抛弃旧的剪贴板图片 OCR 保存链路,避免影响系统剪贴板读取。
- 默认 OCR 主引擎改为 RapidOCR 脚本调用,WeChatOCR 作为备用引擎。
- OCR 脚本输出改为强制 UTF-8 字节输出,避免 Windows 子进程 stdout 编码导致中文乱码。
- RapidOCR 增加小字放大、灰度、对比度、锐化、低置信度过滤和窄范围文本清洗。
- 正式版本号统一为 `1.1.0`。

### 修复

- 修复启用图片识别后前端空白的问题。
- 修复 `Ctrl+Shift+O` 框选时黑屏、闪一下无后续的问题。
- 修复截图内容清楚但 OCR 输出 `ä½ ...` / `Äã...` 等编码乱码的问题。
- 修复 Tauri 打包 MSI 不接受 `1.1.0-beta.1` 预发布版本号的问题。
- 修复安装包未递归包含 `wco_data\Model` 导致备用 WeChatOCR 缺少模型资源的问题。

### 验证

- `npm run build`
- `cargo test --manifest-path src-tauri\Cargo.toml`
- `npm run tauri -- build`
- 本地 NSIS 安装包覆盖安装到 `%LOCALAPPDATA%\ClipNest`
- 已安装目录 RapidOCR 与 WeChatOCR 备用引擎均可对真实截图输出中文

## [1.0.1] - 2026-06-04

稳定化重构版本。修复 v1.0.0 / v1.1.0 撤回后积累的稳定性问题,完成 MVP 闭环。

### 修复 (高优先级)

- **快捷键统一**: 主保存键 `Alt+W` → `Ctrl+Shift+S`, `Alt+W` 降级为可在设置中清空的备用
- **标题长度**: 弹窗标题默认 10 字, 设置页可选 10/20/30, 超出时禁止保存并提示
- **真实剪贴板读取**: 新增 `get_current_clipboard_text` Tauri 命令, 通过 `tauri-plugin-clipboard-manager` 直接读取系统剪贴板, 取代缓存值
- **空剪贴板提示**: 弹窗打开时若剪贴板为空或读取失败, 显式提示"当前剪贴板没有可保存的文本"
- **图片功能关闭**: UI 移除图片相关入口, `save_image` 命令保留但返回明确错误, 数据库 `image_path` 列保留以兼容历史数据
- **数据库迁移安全化**:
  - 新增 `metadata.schema_version` 跟踪
  - 所有 ALTER TABLE 通过 `ensure_column` 字段检查后再执行
  - 启动前自动备份到 `app_data_dir/backups/copyliusq-YYYYMMDD-HHMMSS.db`
  - 迁移日志输出到 stderr (`[db] migrating to schema_version N`)
  - 字段表说明见 `docs/MIGRATION.md`

### 重构

- **后端拆分** (`src-tauri/src/lib.rs` 911 行 → 6 个模块):
  - `commands.rs` — Tauri 命令
  - `clipboard.rs` — 剪贴板读写
  - `hotkeys.rs` — 全局快捷键
  - `tray.rs` — 系统托盘
  - `settings.rs` — Settings 结构 + 读取助手 + 默认值
  - `search.rs` — 搜索 + 过滤
  - `database.rs` — 已存在, 加 `conn_ref` / `fts_search` / `find_by_content`
  - `tags.rs` / `ai.rs` — 已存在
- **前端基础结构** (`src/App.tsx` 1542 行, 拆出 9 个新文件):
  - `src/types.ts` — TypeScript 类型集中
  - `src/lib/api.ts` — 所有 `invoke()` 调用
  - `src/lib/analyze.ts` — 内容类型识别(URL/code/prompt/text),代替 `mockAI.ts`
  - `src/lib/format.ts` — 时间/截断助手
  - `src/lib/highlight.tsx` — 搜索高亮
  - `src/hooks/useSnippets.ts` — 片段列表状态
  - `src/hooks/useToast.ts` — Toast 状态
  - `src/components/Sidebar.tsx` / `Toast.tsx` / `SnippetList.tsx`
  - `src/app/App.tsx` — 入口分发

### 新增

- **设置项**:
  - `capture_shortcut_alt` (备用保存快捷键, 默认 `Alt+W`)
  - `title_max_length` (默认 `10`)
  - `ai_enabled` 默认 `false`(以前默认 `true`)
  - `ollama_model` 默认从 `qwen2.5:3b` → `qwen3:4b`
- **Tauri 命令**:
  - `get_current_clipboard_text` (真实读剪贴板)
  - `get_db_path` / `open_db_dir` / `backup_database` (数据库管理)
  - `save_snippet_force` (覆盖重复检测强制保存)
  - `list_snippets` (带 filter 的列表接口, kind/value/limit/pinned_only)
- **AI 标签改进**:
  - Ollama 超时从 30s 降到 8s
  - Prompt 改中文, 要求 JSON 输出 `{"tags": [...], "summary": "..."}`
  - 使用 Ollama `format: "json"` 选项
  - 失败时静默降级为规则标签(原"回退"配置)
- **测试**:
  - Rust 单元测试从 0 → 7 (settings/load/overrides/clamp/seed; search/limit; empty/recent; schema_version; find_by_content; fts_search/pinned_first)

### 不变更 / 显式放弃

- 图片保存功能 v1.0.1 暂不开放(计划在 v1.1+ 恢复)
- 不做云同步、账号系统、向量数据库、复杂 AI 知识库
- UI 风格保持原样,只做稳定和易用

## [1.0.1-2] - 2026-06-04 (稳定性补强)

在 1.0.1 基础上,做全局一致性修复,使代码与文档完全对齐,作为长期稳定版。

### Changed

- **统一入口**:`src/main.tsx` 现在从 `./app/App` 导入,旧的 `src/App.tsx` (承载业务逻辑) 已删除,所有逻辑迁到 `windows/` 下的三个窗口
- **窗口 label 统一**:后端预创建三个窗口的 label 改为 `capture` / `search` / `settings`,前端 `App.tsx` 同步
- **API 集中化**:所有 Tauri 命令调用封装到 `src/lib/api.ts`,前端不再直接散落 `invoke()`
- **`mockAI.ts` 改名为 `lib/analyze.ts`**,支持 URL / Code / Prompt / Text 四种类型识别(更准的代码语言检测、Prompt 关键词检测)
- **侧边栏分类改为真过滤**:`Sidebar.tsx` 切换分类时调用 `listSnippets` 的 `filterKind` / `filterValue`,不再把分类名塞进搜索框
- **搜索键盘导航**:`useKeyboardNavigation` hook,↑/↓ 切换、Enter 复制关闭、Ctrl+C 复制不关闭、Esc 关闭
- **DetailPanel 独立**:从 SearchView 抽离为 `components/DetailPanel.tsx`,包含完整编辑(标题/标签)、置顶/删除、复制、内容高亮、URL 可点击
- **CSS 增补**:新增 `.detail-*`、`.duplicate-banner*`、`.settings-row-mono`、`.toggle-disabled`、`.secondary-button` 等样式

### Fixed

- **重复保存提示**:`save_snippet` 错误从 `DUPLICATE:id:title` 改为结构化 `DUPLICATE::{"id":N,"title":"..."}`,前端 `parseDuplicateError` 解析,横幅提供"取消 / 打开已有记录 / 仍然保存副本"三选项
- **AI 返回结构**:`auto_tag_ai` 现在返回 `AiTagResult { tags: Vec<String>, summary: String, source: String }`,前端可分别处理标签与摘要(摘要暂不持久化)
- **图片读取完全移除**:`CaptureWindow` 不再调用 `navigator.clipboard.read`,只通过后端 `get_current_clipboard_text`
- **复制走后端**:`navigator.clipboard.writeText` 已替换为 `api.copyToClipboard`(Tauri 后端)
- **设置页版本号**:显示 `1.0.1`,不再出现 `v1.0.0` 或 `Alt+W 主快捷键` 等旧信息
- **设置快捷键**:`Ctrl+Shift+S` 为主、`Alt+W` 为备用(可清空)、`Alt+Space` 为搜索
- **FTS5 触发器**:`INSERT / UPDATE / DELETE` 都正确维护 `snippets_fts`,修改标题或标签后搜索新内容能找到

### 验证

- `npx tsc --noEmit`: 0 错误
- `cargo test --lib`: 7/7 通过
- `npm run build`: 263 KB bundle
- 文档/代码/UI 三方一致(README / SPEC / CHANGELOG / 实际入口)

---

## ⚠️ 撤回公告：v1.1.0 已撤回（2026-06-02）

v1.1.0 已在 2026-06-02 撤回, 原因:
1. 未在开发环境跑通即发布
2. 覆盖安装污染了用户环境(DB 迁移脚本)
3. v1.1.0 的安装包不应被普通用户下载

**给已经装过 v1.1.0 的用户**: v1.0.1 安装包可覆盖装回, DB 自动迁移, 17 条已有数据完整保留。

---

## [1.0.0] - 2026-06-02

首个正式发布版本。详见 v1.0.0 时代的 `SPEC.md` 备份。

