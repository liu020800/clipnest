# ClipNest v1.1 实施计划

## 一、代码审计（v1.0 → v1.1 之间）

### 后端已注册 Tauri 命令（7 个）

| 命令 | 用途 | 状态 |
|------|------|------|
| `copy_to_clipboard` | 写入剪贴板 | ✅ |
| `save_snippet` | 保存片段 | ✅ |
| `search_snippets` | 搜索 | ✅ |
| `get_clipboard_content` | 读取剪贴板 | ✅ |
| `delete_snippet` | 删除 | ✅ |
| `toggle_pin` | 置顶切换 | ✅ |
| `auto_tag_ai` | Ollama 标签 | ✅ 后端就绪 |

### 后端有 DB 函数但未注册 Tauri 命令（隐患）

| DB 函数 | 用途 | Tauri 命令 | 前端调用 |
|---------|------|-----------|---------|
| `update_snippet` | 改标题/标签/置顶 | ❌ 缺失 | `update_clip` ❌ 调用必然失败 |
| `get_all_snippets` | 全量 | ❌ 缺失 | 无 |
| `get_setting` / `save_setting` | 配置读写 | ❌ 缺失 | 无 |

### 前端调用了不存在的命令（**3 个 bug**）

| 调用点 | 命令 | 实际状态 |
|--------|------|---------|
| `App.tsx:737` | `update_clip` | ❌ 不存在 |
| `App.tsx:753` | `update_clip` | ❌ 不存在 |
| `App.tsx:762` | `export_markdown` | ❌ 整个未实现 |

### 未连接 UI 的后端功能

| 功能 | 后端 | UI |
|------|------|------|
| `auto_tag_ai` | ✅ | ❌ 无按钮 |
| `get_setting` / `save_setting` | ✅ | ❌ 无设置页 |
| `update_snippet` | ✅ | ❌ 详情编辑调用错的命令名 |

---

## 二、v1.1 功能规划

### Tier 1：必修 bug 修复（必须做）

#### Task 1.1 注册 `update_snippet` 为 Tauri 命令
- **目标**：让详情面板标题/标签编辑生效
- **改动**：
  - `lib.rs` 新增 `update_snippet_cmd` 命令
  - `lib.rs:132` `invoke_handler` 添加注册
  - 修复 `App.tsx:737,753` 调用名 `update_clip` → `update_snippet`
- **验收**：详情面板点标题修改 → 回车 → 数据库更新
- **风险**：低（DB 函数已存在并测试过）

#### Task 1.2 实现 Markdown 导出
- **目标**：用户能从侧栏「导出 Markdown」导出全部片段
- **改动**：
  - `lib.rs` 新增 `export_markdown` 命令
  - 路径：`%USERPROFILE%\Documents\ClipNest\export-YYYYMMDD-HHMMSS.md`
  - 文件格式：每个片段 `# 标题` + 元信息 + 标签 + 内容（代码块）
- **验收**：侧栏点导出 → 文件出现在 Documents → 内容含所有片段
- **风险**：低

#### Task 1.3 保存窗口加 `[+ AI]` 按钮
- **目标**：用户能直接调用 Ollama 生成标签
- **改动**：
  - `App.tsx` CapturePopup 在标签行加按钮
  - 点击后调用 `invoke("auto_tag_ai", { title, content })`
  - 成功后 merge 到 `selectedTags` 列表
  - 失败 toast 提示「Ollama 未启动」
- **验收**：复制一段文本 → Alt+W → 点 [+ AI] → 标签自动填入
- **风险**：中（依赖 Ollama 外部服务）

### Tier 2：高价值新功能

#### Task 2.1 设置窗口
- **目标**：统一管理配置项（开机自启、快捷键等）
- **改动**：
  - 新建 `SettingsWindow` 组件（独立第三个窗口）
  - 侧栏「设置」点击 → 打开设置窗口
  - 设置项：开机自启、AI 提供商、模型名、主题
- **验收**：侧栏点设置 → 弹出设置窗口 → 修改配置并保存
- **风险**：中

#### Task 2.2 开机自启
- **目标**：安装时默认勾选，用户可取消
- **改动**：
  - `Cargo.toml` 添加 `tauri-plugin-autostart = "2"`
  - `lib.rs` 注册插件
  - 设置窗口加开关
  - 配置存储：使用 `tauri-plugin-autostart` 的 API
- **验收**：设置窗口勾选 → 重启 Windows → ClipNest 自动在托盘运行
- **风险**：中（Windows 注册表操作）

#### Task 2.3 图片剪贴板支持
- **目标**：能保存剪贴板里的图片（截图工具的图像）
- **改动**：
  - `database.rs` 加 `image_data BLOB` 字段（迁移用 `let _ = ...` 静默处理）
  - `save_snippet` 命令接受 `image_bytes: Option<Vec<u8>>`
  - `get_clipboard_content` 返回 `enum` (text | image)
  - 前端展示图片缩略图
- **验收**：截图 → Alt+W → 看到图片预览 → 保存 → 详情面板能查看图片
- **风险**：中-高（图片 base64 / BLOB 处理）

### Tier 3：暂缓（v1.2+）

- 向量搜索（sqlite-vec）
- 自然语言查询
- 多设备同步
- 标签管理页
- macOS / Linux

---

## 三、依赖图

```
Tier 1:
  Task 1.1 (update_snippet Tauri cmd) ─── 独立 ────────────────────┐
  Task 1.2 (export_markdown)           ─── 独立 ──────────────────┤
  Task 1.3 (AI 按钮)                   ─── 独立 ──────────────────┤
                                                                  │
Tier 2:                                                          │
  Task 2.1 (SettingsWindow)            ─── 独立 ──────────────────┤
  Task 2.2 (autostart)                 ─── 依赖 2.1 (设置窗口)  ─┤
  Task 2.3 (image)                     ─── 独立 ──────────────────┘
```

Tier 1 三项可并行（独立）。Tier 2 中 2.1 → 2.2 有依赖；2.3 独立。

---

## 四、阶段切分与验证

### 阶段 A：Tier 1 全部完成
- [ ] 详情面板编辑生效
- [ ] Markdown 导出可用
- [ ] 保存窗口 [+ AI] 可点击
- [ ] 端到端测试通过
- [ ] **检查点**：决定是否直接发 v1.1.0，或继续做 Tier 2

### 阶段 B：Tier 2 全部完成
- [ ] 设置窗口可打开
- [ ] 开机自启可用
- [ ] 图片保存可用
- [ ] 端到端测试通过
- [ ] **检查点**：发 v1.1.0

---

## 五、风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Ollama 不存在时 AI 按钮崩溃 | 中 | 失败时 toast，不阻塞保存流程 |
| Markdown 导出大文件卡 UI | 中 | 后台异步 + 进度提示 |
| 图片 BLOB 让 DB 膨胀 | 中 | 默认压缩 1920px / 限单张 5MB |
| 开机自启权限被拒 | 低 | 设置窗口明确显示失败原因 |
| 设置窗口与 save/search 路由冲突 | 中 | 第三个独立窗口 `label = "settings"` |

---

## 六、成功标准（v1.1 整体）

- 所有 v1.0 已存在的 UI 元素 100% 可用（无 broken 按钮）
- 至少完成 Tier 1（3 个 bug 修复）
- 如果时间和精力允许，完成 Tier 2.1 + 2.2（设置窗口 + 开机自启）
- 全部改动有端到端验证
- 新的 GitHub Release 附带更新后的 NSIS + MSI 安装包

---

## 七、范围确认

需要你确认：

1. **Tier 1 必做** — 同意？
2. **Tier 2 范围** — 全部做 / 只做 2.1+2.2 / 都不做（只发 v1.1 bugfix）
3. **是否需要"图片剪贴板"** — 这是 Tier 2 中最复杂的，建议先做 Tier 1 看看
