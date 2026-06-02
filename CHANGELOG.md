# 更新日志

所有重要变更记录在此文件。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

---

## [1.1.0] - 2026-06-02

聚焦「闭环 + 体验」的增量更新：修复 v1.0 遗留的 3 个已实现但未接通的功能，并扩展为完整的设置中心与图片捕获能力。

### 新增

- **设置窗口**
  - 托盘菜单 + 搜索侧栏均可打开（`Alt+Space` → 侧栏齿轮）
  - 「开机自启」开关（基于 `tauri-plugin-autostart`，写 Windows 注册表 Run 项）
  - 知识库统计（总数 / 固定数）
  - 一键导出 Markdown
  - 版本信息 + GitHub 链接
- **图片剪贴板支持**
  - 保存窗口自动检测 PNG / JPG 图片
  - 图片以二进制形式保存到 `%APPDATA%\com.copyliusq.desktop\images\`
  - 数据库新增 `image_path` 列（向后兼容，存量库静默 ALTER）
  - 详情面板未来可预览（v1.2 计划）
- **AI 标签按钮**：保存窗口「标签」section 右上角新增 `[AI 标签]` 按钮，调用本地 Ollama (`qwen2.5:3b`) 生成补充标签并自动合并

### 修复

- 详情面板标题/标签编辑失败（`update_clip` 命令缺失 → 注册 `update_snippet` 命令 + 修复前端调用）
- 详情面板「导出」按钮（`export_markdown` 命令缺失 → 完整实现：导出到 `%USERPROFILE%\Documents\ClipNest\export-YYYYMMDD-HHMMSS.md`）
- 保存窗口无法触发 AI 标签（新增 UI 入口）

### 技术

- 新增 Tauri 命令：`update_snippet`、`export_markdown`、`open_settings`、`get_all_settings`、`save_setting`、`get_autostart`、`set_autostart`、`save_image`
- 新增 Tauri 插件：`tauri-plugin-autostart`（v2）
- 新增 Rust crate：`base64`
- 新增 DB 表：`settings (key, value)`
- 新增 DB 列：`snippets.image_path TEXT`
- 前端新增组件：`SettingsWindow`，按 window label 路由
- 前端 CSS：新增 ~180 行（设置窗口样式 + 开关 / 按钮 / 卡片）

---

## [1.0.0] - 2026-06-02

首个正式发布版本。

### 新增

- **全局快捷键**
  - `Alt+W` 打开保存窗口
  - `Alt+Space` 打开搜索窗口
- **剪贴板捕获**：自动读取当前剪贴板内容、生成标题、识别类型、打标签
- **保存窗口 UI**：标题编辑、标签可视化选择、内容预览、复制原文、键盘快捷操作
- **搜索窗口 UI**：三栏 ClipNest 风格（侧栏分类 + 内容列表 + 详情面板）
- **全文搜索**：标题 / 内容 / 标签 / 拼音 联合搜索
- **拼音搜索**：中文内容自动生成 pinyin 索引，支持 `zhongwen` 搜「中文」
- **模糊搜索**：子串匹配，多词空格分隔
- **分类筛选**：全部 / AI / NAS / 代码 / 教程 / 网址 / 图片
- **详情面板**：查看完整内容、编辑标题、编辑标签、置顶、删除、一键复制
- **系统托盘**：左键打开搜索、右键菜单（固定收藏 / 最近保存 / 退出）
- **AI 标签**：本地 Ollama HTTP 客户端（v1.1 计划集成到 UI）
- **强杀保护**：`tauri-plugin-single-instance` + `RunEvent::ExitRequested::prevent_exit`
- **数据安全**：SQLite DELETE journal 模式（无 WAL，强杀进程零数据丢失）
- **安装包**：NSIS (.exe) + MSI 两种格式

### 技术

- 桌面框架：**Tauri 2**（单 exe 13.8 MB）
- 前端：**React 19 + TypeScript + Tailwind CSS v4**
- 后端：**Rust**（rusqlite + FTS5 + pinyin + reqwest）
- 数据库：SQLite，含 `snippets` 表 + `snippets_fts` 全文索引 + 3 触发器
- 窗口：save (560×500) + search (900×580)，按 label 路由前端
- 主题：深色玻璃风，CLI 风格键盘芯片

### 修复

- 启动时 `ALTER TABLE ADD COLUMN pinyin` 重复执行报错（用 `let _ =` 静默处理）
- SQLite WAL 模式在强杀场景下导致数据丢失（切换为 DELETE journal）
- 剪贴板同步 FIFO 中 UTF-8 字符导致 panic（`chars().take(27)` 字符级截断）
- 中文搜索失败（`normalize_search_term` 过滤掉非 ASCII 字符）
- 标签子串误匹配（`contains_word` 加入 ASCII 词边界、CJK 子串双策略）
- 托盘菜单标题 UTF-8 截断 panic
- 保存窗口关闭导致应用退出（`.close()` → `.hide()`）
- Ctrl+S 冲突（原 `Ctrl+Shift+S` 改为 `Alt+W`）

### 已知问题

- 详情面板的标题/标签编辑（`update_clip` 命令）尚未注册到 `invoke_handler`，调用会失败（v1.1 修复）
- 导出 Markdown 命令（`export_markdown`）已实现但 UI 未集成
- `auto_tag_ai` 命令可用但保存窗口未提供 UI 触发（v1.1 修复）

---

## [0.1.0] - 2026-05 (MVP)

内部 MVP 版本，未对外发布。
