# ClipNest v1.1.2

> Windows 本地剪贴板知识管理工具 — 支持文本保存、剪贴板历史粘贴、全文搜索、透明框选 OCR。隐私优先,数据存在本地 SQLite。

## 核心闭环

```
复制文本 → 自动进入历史
              ↓
       随时按 Alt+Space
              ↓
       ↑/↓ 选中 → Enter / 双击 → 粘贴到原窗口

需要手动整理时: Ctrl+Shift+S → 输入标题(默认 10 字) → Enter → 已保存
```

## 快捷键

| 操作 | 主快捷键 | 备用/说明 |
|---|---|---|
| 保存当前剪贴板 | **Ctrl+Shift+S** | Alt+W(可在设置中清空) |
| 打开搜索 | **Alt+Space** | 焦点自动落在搜索框 |
| 框选屏幕 OCR | **Ctrl+Shift+O** | 透明全屏框选,识别后进入保存窗口 |
| 搜索窗口关闭 | **Esc** | 失焦自动关闭(150ms 延迟) |
| 搜索上下移动 | **↑ / ↓** | 循环切换 |
| 搜索粘贴历史 | **Enter** | 写回剪贴板并粘贴到原窗口 |
| 搜索双击粘贴 | 双击条目 | 与 Enter 行为一致 |
| 搜索复制不关闭 | **Ctrl+C** | 适合连续复制多条 |

## 主要功能

- **四类自动识别**:URL / 代码 / Prompt / 文本 — 自动选最合适的标题
- **FTS5 全文搜索**:标题、正文、标签、拼音四列索引;`#docker` 标签过滤;LIKE 兜底
- **剪贴板历史粘贴**:自动记录文本剪贴板历史,搜索选中后可直接粘贴到原窗口
- **45 条内置规则**:保存时自动打规则标签(`auto_tag_on_capture` 可关)
- **可选 AI 标签**:Ollama 8 秒超时,失败自动回退到规则
- **重复检测**:同一内容保存会弹出横幅,可选"仍然保存副本"或"打开已有记录"
- **Markdown 导出**:保存到 `Documents\ClipNest\clipnest-export-<时间戳>.md`
- **JSON 导入/导出**:用于备份和跨设备迁移
- **标签管理**:重命名 / 合并 / 删除,操作影响行数即时反馈
- **屏幕框选 OCR**:`Ctrl+Shift+O` 框选屏幕文本,本地识别后进入保存窗口
- **手动备份数据库**:一键生成 `copyliusq-YYYYMMDD-HHMMSS.db`
- **开机自启**(可选)

## 数据存储

- **数据库位置**:`%APPDATA%\com.copyliusq.desktop\copyliusq.db`
- **备份目录**:`%APPDATA%\com.copyliusq.desktop\backups\`
- **应用卸载后**:数据库与备份**不会被自动删除**,留在 `AppData\Roaming` 目录。重装后可继续使用。

## 设置项

| 设置 | 默认值 | 范围 | 说明 |
|---|---|---|---|
| `autostart` | `false` | bool | 开机自启 |
| `capture_shortcut` | `Ctrl+Shift+S` | 字符串 | 保存快捷键(主) |
| `capture_shortcut_alt` | `Alt+W` | 字符串(空=禁用) | 保存快捷键(备) |
| `search_shortcut` | `Alt+Space` | 字符串 | 搜索快捷键 |
| `screen_ocr_shortcut` | `Ctrl+Shift+O` | 字符串(空=禁用) | 框选 OCR 快捷键 |
| `clipboard_history_enabled` | `true` | bool | 自动记录文本剪贴板历史 |
| `clipboard_history_max` | `500` | 50-5000 | 自动历史保留条数 |
| `title_max_length` | `10` | 10/20/30 | 标题最大字数 |
| `search_limit` | `50` | 1-500 | 单次搜索最大结果数 |
| `search_debounce_ms` | `150` | 0-2000 | 搜索输入防抖(ms) |
| `auto_close_on_blur` | `true` | bool | 失焦自动关闭搜索窗 |
| `auto_close_delay_ms` | `150` | 0-5000 | 失焦延迟(ms) |
| `auto_tag_on_capture` | `true` | bool | 保存时自动规则打标签 |
| `capture_text_max_length` | `50000` | 100-1000000 | 单条最大字符数 |
| `markdown_export_pinned_only` | `false` | bool | 导出仅含已固定 |
| `ai_enabled` | `false` | bool | 启用 AI 标签(默认关闭) |
| `ollama_endpoint` | `http://localhost:11434` | 字符串 | Ollama 服务地址 |
| `ollama_model` | `qwen3:4b` | 字符串 | Ollama 模型名 |
| `ai_tag_fallback` | `rules` | rules/none | AI 失败时回退 |

## v1.0.1 → v1.1.2 主要变化

详见 [CHANGELOG.md](CHANGELOG.md) 和 [docs/MIGRATION.md](docs/MIGRATION.md)。

- 新增文本剪贴板历史自动记录和历史内容直接粘贴
- 修复开机自启后前端白屏问题
- 新增透明框选 OCR 窗口和 `Ctrl+Shift+O` 快捷键
- 抛弃早期剪贴板图片 OCR 路线,避免影响剪贴板读取
- OCR 输出强制 UTF-8,修复中文乱码
- RapidOCR 增加小字预处理、置信度过滤和窄范围文本清洗
- 安装包内置 WeChatOCR 备用主机与模型资源
- 版本号正式稳定为 `1.1.2`
- 修复 `v1.1.0` 中保存文本复制失败的问题

## 安全与隐私

- 全部数据存在本地 SQLite,**不上传任何数据**
- 不做账号系统、不做云同步、不接任何外部 API(除非用户主动启用 AI 标签并配置 Ollama)
- OCR 默认走本机 Python + RapidOCR;备用 WeChatOCR 资源随安装包内置,不调用用户正在登录的微信客户端
- 应用卸载不会自动删除数据库

## 开发

```bash
# 安装依赖
npm install

# 开发模式(Vite HMR + Rust 热编译)
npm run tauri dev

# 类型检查 + 生产构建前端
npm run build

# 完整打包(Nsis 安装包)
npm run tauri build
```

Rust 单元测试:
```bash
cd src-tauri && cargo test --lib
```

## 手动测试步骤

见 [docs/TESTING.md](docs/TESTING.md)。

## 架构

- **后端**(`src-tauri/src/`):`lib.rs` 启动器 + 模块 `commands.rs` / `clipboard.rs` / `hotkeys.rs` / `tray.rs` / `settings.rs` / `search.rs` + `database.rs` / `ai.rs` / `tags.rs` / `ocr/`
- **前端**(`src/`):`main.tsx` 入口,`app/App.tsx` 窗口分发,`windows/` 四个窗口实现,`components/` + `hooks/` + `lib/` 工具集

## 已知问题 / 计划中

- **图片保存**:当前版本不做图片入库保存,只支持框选 OCR 后保存识别文字。DB 图片字段保留以兼容历史数据。
- **AI 摘要**:目前 Ollama 返回的 `summary` 字段不持久化到数据库(避免引入 schema 迁移)。仅在保存时显示在 UI 上一瞬。
- 视频/富文本预览不支持
- 多语言界面目前仅中文

## 许可

仅供个人使用。
