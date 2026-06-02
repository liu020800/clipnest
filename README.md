# ClipNest v1.0

> **个人碎片知识捕获系统** — Windows 本地运行，剪贴板内容一键收纳，全文搜索随时调用。

![Version](https://img.shields.io/badge/version-1.0.0-4cc2ff)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
![License](https://img.shields.io/badge/license-MIT-green)

---

## 目录

- [这是什么](#这是什么)
- [核心特性](#核心特性)
- [快速开始](#快速开始)
- [详细使用说明](#详细使用说明)
  - [1. 快速保存](#1-快速保存)
  - [2. 知识库浏览](#2-知识库浏览)
  - [3. 详情与编辑](#3-详情与编辑)
  - [4. 系统托盘](#4-系统托盘)
  - [5. AI 自动标签](#5-ai-自动标签可选)
- [快捷键参考](#快捷键参考)
- [数据存储](#数据存储)
- [常见问题](#常见问题)
- [技术架构](#技术架构)
- [开发者指南](#开发者指南)
- [路线图](#路线图)
- [许可证](#许可证)

---

## 这是什么

ClipNest 是一款 Windows 桌面端剪贴板知识管理工具。它解决一个具体场景：

> **你每天复制几十段文字**（代码片段、命令、文章段落、待办事项、灵感、链接），
> 但 Windows 自带的剪贴板历史只能存 25 条且重启清空。
> 想要的东西总是找不到。

ClipNest 的做法：

1. **任意时刻按 `Alt+W`** → 当前剪贴板内容自动捕获并预览
2. **回车** → 永久存入本地 SQLite 数据库，带自动标题/标签/分类
3. **任意时刻按 `Alt+Space`** → 模糊搜索 + 拼音搜索 + 全文检索，立刻调出

整个过程不到 2 秒，**不打断当前工作流**。

### 与同类工具对比

| 工具 | 存储 | AI 摘要 | 拼音搜索 | 全文搜索 | 单文件 |
|------|------|---------|----------|----------|--------|
| Windows 剪贴板历史 | ❌ 内存 | ❌ | ❌ | ❌ | 内置 |
| Ditto | ✅ 本地 | ❌ | ❌ | ✅ | ✅ |
| 1Clipboard | ❌ 云端 | ❌ | ❌ | ✅ | ❌ |
| **ClipNest** | ✅ 本地 | ✅ Ollama | ✅ | ✅ | ✅ |

---

## 核心特性

### 1. 一键捕获剪贴板

- 全局快捷键 `Alt+W`，不抢当前焦点
- 自动从内容截取标题，规则智能识别
- 自动内容分析（URL / 代码 / 图片 / 文本）
- 自动标签（基于规则 + 可选 AI）
- 实时预览所保存的内容，所见即所得

### 2. 极速知识库

- 三栏式 ClipNest 风格界面：分类导航 / 列表 / 详情
- 全局搜索（标题 + 内容 + 标签 + 拼音）
- 拼音搜索：输入 `zhongwen` 匹配「中文」
- 模糊搜索：输入 `docker` 匹配所有含 docker 的记录
- 分类筛选：全部 / AI / NAS / 代码 / 教程 / 网址 / 图片
- 详情可编辑标题和标签
- 置顶 / 取消置顶
- 一键复制原内容

### 3. 系统托盘

- 左键点击托盘 → 打开搜索
- 右键菜单：
  - 打开搜索 (Alt+Space)
  - 固定收藏（最多 5 条置顶内容）
  - 最近保存（最近 5 条）
  - 退出

### 4. AI 自动标签（可选）

- 本地 Ollama 集成，**数据不外传**
- 一键调用本地 LLM 分析内容生成 3-5 个标签
- 失败时降级为规则标签

### 5. 永久本地存储

- SQLite + FTS5 全文索引
- 单文件 `copyliusq.db` 即可备份/迁移
- 拼音索引（中文搜索友好）
- DELETE journal 模式（强杀进程不丢数据）

---

## 快速开始

### 系统要求

- **操作系统**: Windows 10 (1809+) / Windows 11
- **架构**: x64
- **依赖**: 无（打包时已内嵌 WebView2）

### 安装

1. 从 [Releases](releases/) 下载最新安装包：
   - **`ClipNest_1.1.0_x64-setup.exe`** — NSIS 安装包（推荐，普通用户）
   - `ClipNest_1.1.0_x64_en-US.msi` — MSI 安装包（适合企业批量部署）

2. 双击安装包，按向导完成。

3. 启动后**无窗口**（正常），程序在系统托盘运行，图标 🗂️ 出现在右下角。

4. 按 `Alt+Space` 打开搜索窗口，或 `Alt+W` 开始保存。

> v1.1.0 新增：托盘菜单 → 设置，可开启「开机自启」

### 卸载

- **设置 → 应用 → 搜索 ClipNest → 卸载**，或
- 重新运行安装包选择 **Remove**

> 卸载**不会**删除数据库文件（`%APPDATA%\com.copyliusq.desktop\copyliusq.db`），如需清理请手动操作。

### 验证安装

1. 复制任意文本（例如浏览器里选中一段复制）
2. 按 `Alt+W`
3. 应该看到屏幕中央弹出保存窗口，预览框内显示刚复制的内容
4. 按 `Enter` 保存
5. 按 `Alt+Space` 打开搜索，能看到刚才保存的记录

---

## 详细使用说明

### 1. 快速保存

**触发方式**：复制任意文本 → 按 `Alt + W`

**界面**：

```
┌─────────────────────────────────────────┐
│ 保存剪贴板             [Esc 取消]       │
│ 确认后保存到本地知识库                   │
├─────────────────────────────────────────┤
│ 名称                            18 / 30 │
│ ┌─────────────────────────────────────┐ │
│ │ Docker容器启动命令                  │ │
│ └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│ ✦ 内容摘要                             │
│ 这是一个 Docker 容器启动命令，适合 NAS... │
├─────────────────────────────────────────┤
│ 标签                                    │
│ [#Docker] [#NAS] [#命令行]               │
│ [+ 自定义]                               │
├─────────────────────────────────────────┤
│ 预览            [代码片段]  [📋 复制]    │
│ ┌─────────────────────────────────────┐ │
│ │ docker run -d --name nginx \        │ │
│ │   -p 80:80 -v /data:/data nginx     │ │
│ │ ...                                 │ │
│ └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│ ↵ 保存到知识库    [取消]   [保  存]      │
└─────────────────────────────────────────┘
```

**字段说明**：

| 字段 | 说明 | 可编辑 |
|------|------|--------|
| 名称 | 自动从内容截取，最多 30 字符 | ✅ |
| 内容摘要 | 智能分析（URL/代码/文本/图片） | ❌ 自动 |
| 标签 | 自动打的标签，可点击取消 | ✅ |
| 预览 | 实际保存的完整内容 | ❌ 只读 |
| 类型 pill | 内容类型：代码片段/网址/图片/普通文本 | — |

**操作**：

| 按键 | 行为 |
|------|------|
| `Enter` | 保存到知识库，弹窗关闭 |
| `Esc` | 取消保存，弹窗关闭 |
| 点击「保存」 | 同 Enter |
| 点击「取消」 | 同 Esc |
| 失焦 | 弹窗自动隐藏（不保存） |
| 点击「复制」 | 复制原文到剪贴板 |
| 点击标签 | 取消该标签 |

**自动标题规则**（按优先级）：

1. URL 类型 → 使用域名（如 `github.com`）
2. 代码类型 → 提取首个英文关键词
3. 文本类型 → 截取首个句子/分句
4. 超过 28 字符则截断

**自动标签规则**：

- 关键词匹配（`docker` → `#Docker`，`python` → `#Python`）
- 涵盖 100+ 中英文技术词汇
- 最多 3 个标签

---

### 2. 知识库浏览

**触发方式**：按 `Alt + Space`

**界面**：

```
┌──────────┬────────────┬──────────────────────┐
│ ClipNest │  🔍 docker  │ Docker容器启动命令  │
│          ├────────────┤ 代码片段             │
│ 全部 (12) │────────────│ [#Docker][#NAS]     │
│ 最近保存  │ [Card 1]  │ ──────────────      │
│ ──────── │ [Card 2]  │ 原始内容    [📋 复制]│
│ AI (3)   │ [Card 3]  │ ┌──────────────────┐ │
│ NAS (2)  │ [Card 4]  │ │ docker run...   │ │
│ 代码 (5) │            │ └──────────────────┘ │
│ 教程 (1) │            │ ──────────────      │
│ ──────── │            │ ✦ 内容分析 自动生成  │
│ 网址 (4) │            │ 这是一个 Docker 容  │
│ 图片 (1) │            │ 器启动命令...        │
│          │            │ ──────────────      │
│ ──────── │            │ [📌 置顶] [🗑 删除]  │
│ 设置      │            │                      │
└──────────┴────────────┴──────────────────────┘
```

**侧栏分类**：

| 分类 | 含义 |
|------|------|
| 全部 | 所有保存的记录 |
| 最近保存 | 按时间倒序的全部记录 |
| AI | 内容分析含 `ai` 关键词 |
| NAS | 内容分析含 `nas` 关键词 |
| 代码 | 内容是代码 |
| 教程 | 内容含教程关键词 |
| 网址 | URL 类型 |
| 图片 | 图片类型 |

**搜索框**：

- 输入任意字符 → 实时过滤
- 支持标题 / 内容 / 标签 / 拼音
- 150ms 防抖
- 按分类时自动填入分类对应的搜索词

**列表卡片**：

- 类型图标 + 标题（1 行截断）+ 时间
- 摘要预览（2 行）
- 前 3 个标签 + `+N` 提示
- 点击 → 加载到详情面板

---

### 3. 详情与编辑

**操作位置**：右侧详情面板

**可编辑**：

| 元素 | 操作 | 说明 |
|------|------|------|
| 标题 | 点击标题 → 修改 → 回车 | 立即更新数据库 |
| 标签 | 点击标签区 → 修改 → 回车 | 逗号分隔多个 |
| 置顶 | 点击「📌 置顶」 | 状态切换，托盘同步 |
| 删除 | 点击「🗑 删除」 | 立即删除，托盘同步 |
| 复制 | 点击「📋 复制」 | 原内容写入剪贴板 |

**操作反馈**：所有操作都会在右下角弹出 toast 提示。

---

### 4. 系统托盘

**位置**：任务栏右下角 🗂️ 图标

**操作**：

| 行为 | 结果 |
|------|------|
| 左键单击 | 打开搜索窗口 |
| 右键单击 | 弹出菜单 |

**菜单项**：

```
┌────────────────────────────┐
│ 打开搜索         Alt+Space │
├────────────────────────────┤
│ ▶ 固定收藏                │
│   📌 Docker启动命令        │
│   📌 Linux常用指令         │
│   📌 Python脚本            │
├────────────────────────────┤
│ ▶ 最近保存                │
│   部署笔记                │
│   Docker compose 模板     │
├────────────────────────────┤
│ 退出                      │
└────────────────────────────┘
```

- 点击「固定收藏」/「最近保存」子菜单项 → 复制该内容到剪贴板
- 托盘菜单在保存/删除/置顶时自动刷新

---

### 5. AI 自动标签（可选）

> 需要本地 Ollama，未安装时跳过此节。

**前置**：

```powershell
# 1. 安装 Ollama
# https://ollama.com/

# 2. 拉取模型（推荐 qwen2.5:3b，~2GB）
ollama pull qwen2.5:3b

# 3. 启动 Ollama（通常装完自启）
ollama serve
```

**使用**：在保存窗口点 **[+ AI]** 按钮（如已集成）或通过 Tauri 命令调用：

```typescript
const tags = await invoke("auto_tag_ai", {
  title: "Docker命令",
  content: "docker run -d --name nginx ..."
});
```

**失败处理**：若 Ollama 未运行或模型不存在，命令返回错误，规则标签仍可用。

---

## 快捷键参考

| 快捷键 | 功能 | 适用窗口 |
|--------|------|----------|
| `Alt + W` | 打开保存窗口（自动捕获剪贴板） | 全局 |
| `Alt + Space` | 打开/聚焦搜索窗口 | 全局 |
| `Enter` | 保存 / 触发 | 保存/搜索 |
| `Esc` | 取消 / 隐藏 | 保存/搜索 |
| 失焦 | 自动隐藏 | 保存/搜索 |
| 鼠标点击托盘（左） | 打开搜索 | 托盘 |

---

## 数据存储

### 数据库位置

```
%APPDATA%\com.copyliusq.desktop\copyliusq.db
```

展开后：`C:\Users\<用户名>\AppData\Roaming\com.copyliusq.desktop\copyliusq.db`

### 表结构

```sql
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,           -- 标题
    content TEXT NOT NULL,          -- 内容
    pinyin TEXT DEFAULT '',         -- 拼音索引（自动生成）
    type TEXT,                      -- text / code / url
    tags TEXT,                      -- 逗号分隔的标签
    source_app TEXT,                -- 来源应用（预留）
    created_at DATETIME,            -- 创建时间
    updated_at DATETIME,            -- 更新时间
    pinned INTEGER DEFAULT 0        -- 是否置顶
);

CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title, content, tags, pinyin,
    content=snippets, content_rowid=id
);
```

### 备份与迁移

数据库是**单文件**，备份只需复制：

```powershell
# 备份
Copy-Item "$env:APPDATA\com.copyliusq.desktop\copyliusq.db" "D:\backup\copyliusq-$(Get-Date -Format 'yyyyMMdd').db"

# 迁移到新电脑
Copy-Item "D:\backup\copyliusq.db" "$env:APPDATA\com.copyliusq.desktop\"
```

### 隐私

- **所有数据存储在本地**，不联网（除可选的 Ollama）
- 无遥测、无上报
- 应用商店图标、版本号等元数据来自 Tauri 标准打包

---

## 常见问题

### Q: 启动后看不到主窗口？

**正常行为。** ClipNest 是托盘应用，启动后无主窗口。按 `Alt+Space` 打开搜索，或 `Alt+W` 开始保存。托盘图标在右下角通知区域。

### Q: Alt+W 没反应？

按以下顺序排查：

1. 检查是否有其他程序占用 `Alt+W`（如输入法切换、IDE 快捷键）
2. 打开系统托盘，看 ClipNest 图标是否在
3. 如果托盘没有图标，从「开始菜单 → ClipNest」重启
4. 检查 Windows Defender 是否阻止了全局快捷键

### Q: Alt+Space 被系统占用（输入法切换）？

修改 `src-tauri\src\lib.rs:255` 和 `src-tauri\src\lib.rs:281` 中的快捷键，重新编译。

### Q: 保存后搜索不到？

1. 确认是否真的按了 Enter（按 Esc 会取消）
2. 关键词是否完全匹配（搜索是子串匹配）
3. 中文请尝试用拼音搜索（输入 `zhongwen` 搜「中文」）
4. 数据库文件是否损坏：`%APPDATA%\com.copyliusq.desktop\`

### Q: 想要开机自启？

目前未内置此功能。可手动实现：

**方法一**：将 `ClipNest.lnk`（安装目录）放入 `shell:startup` 文件夹
**方法二**：任务计划程序 → 创建任务 → 触发器「登录时」→ 操作「启动程序」

### Q: 数据会丢失吗？

- **强杀进程不会丢**：使用 DELETE journal 模式，提交即写盘
- **卸载不会丢**：数据库独立于程序目录
- **手动删除 `%APPDATA%\com.copyliusq.desktop\` 会丢**

### Q: AI 标签调用失败？

1. 确认 Ollama 运行中（`curl http://localhost:11434`）
2. 确认模型已下载（`ollama list`）
3. 防火墙是否阻止本地 11434 端口
4. UI 上 `[+ AI]` 按钮暂未集成，需要通过 Tauri 命令手动调用（v1.1 计划集成）

### Q: 怎么修改默认快捷键？

编辑 `src-tauri\src\lib.rs`：

```rust
// 保存窗口快捷键 (lib.rs:281)
Shortcut::new(Some(Modifiers::ALT), Code::KeyW)

// 搜索窗口快捷键 (lib.rs:255)
Shortcut::new(Some(Modifiers::ALT), Code::Space)
```

修改后重新 `npm run tauri build`。

### Q: 多显示器下窗口错位？

目前窗口居中于主显示器。如有问题，编辑 `show_search_window` / `handle_save_shortcut` 移除 `.center()`。

---

## 技术架构

```
┌─────────────────────────────────────────────┐
│              Tauri 2 (Rust)                  │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐    │
│  │Global   │  │ Tray +   │  │ Window   │    │
│  │Shortcut │  │ Menu     │  │ Manager  │    │
│  └─────────┘  └──────────┘  └──────────┘    │
│  ┌──────────────────────────────────────┐    │
│  │  SQLite (rusqlite bundled) + FTS5    │    │
│  │  • Pinyin index  • Triggers          │    │
│  │  • DELETE journal (no WAL)           │    │
│  └──────────────────────────────────────┘    │
│  ┌──────────────────────────────────────┐    │
│  │  Ollama HTTP Client (optional)       │    │
│  └──────────────────────────────────────┘    │
├─────────────────────────────────────────────┤
│           React 19 + TypeScript              │
│  ┌──────────────┐  ┌──────────────────┐     │
│  │ CapturePopup │  │  MainLibrary     │     │
│  │  (save win)  │  │  (search win)    │     │
│  │              │  │  ┌────────────┐  │     │
│  │  - Title     │  │  │  Sidebar   │  │     │
│  │  - Tags      │  │  ├────────────┤  │     │
│  │  - Preview   │  │  │ ContentList│  │     │
│  │  - Buttons   │  │  ├────────────┤  │     │
│  │              │  │  │ DetailPanel│  │     │
│  └──────────────┘  └──────────────────┘     │
├─────────────────────────────────────────────┤
│         Tailwind CSS v4 (Dark Glass)         │
└─────────────────────────────────────────────┘
```

### 技术选型理由

| 选择 | 原因 |
|------|------|
| Tauri 2 | 相比 Electron，包体积小 10x（13MB vs 150MB+），启动快，内存低 |
| Rust 后端 | SQLite 性能 + 强类型 + 单一可执行文件 |
| React 19 | 组件化 + 生态成熟 |
| SQLite + FTS5 | 嵌入式、无需服务器、毫秒级全文搜索 |
| Tailwind v4 | 快速原型 + 零运行时 CSS-in-JS 开销 |
| 单实例插件 | 防止多开导致剪贴板冲突 |

### 关键设计

1. **两个独立窗口**：save (无标题栏, 置顶) + search (有完整 UI)，通过 `getCurrentWindow().label` 路由
2. **DELETE journal 模式**：牺牲 ~10% 写入性能换取强杀进程零数据丢失
3. **Unicode-aware 搜索**：CJK 字符不进行分词，直接子串匹配
4. **拼音索引**：保存时生成 pinyin 列，搜索时同时匹配拼音
5. **托盘菜单动态刷新**：保存/删除/置顶后自动重建菜单
6. **mt-auto 按钮布局**：保证窗口高度变化时按钮始终贴底

---

## 开发者指南

### 构建环境

- Node.js 20+
- Rust 1.70+ (安装到 `C:\Users\51510\.cargo\bin`)
- Windows 10/11 + WebView2

### 开发模式

```powershell
$env:Path = "C:\Users\51510\.cargo\bin;" + $env:Path
cd H:\code\copyliusq
npm install
npm run tauri dev
```

### 发布构建

```powershell
$env:Path = "C:\Users\51510\.cargo\bin;" + $env:Path
cd H:\code\copyliusq
npm run tauri build
```

输出位置：
- 单 exe: `src-tauri\target\release\copyliusq.exe` (13.8 MB)
- NSIS 安装包: `src-tauri\target\release\bundle\nsis\ClipNest_*_x64-setup.exe` (3.4 MB)
- MSI 安装包: `src-tauri\target\release\bundle\msi\ClipNest_*_x64_en-US.msi` (5.0 MB)

### 项目结构

```
H:\code\copyliusq\
├── src/                          # React 前端
│   ├── App.tsx                   # 主组件（按 window label 路由）
│   ├── mockAI.ts                 # 内容分析（本地）
│   ├── main.tsx                  # React 入口
│   └── index.css                 # 全局样式 + Tailwind v4
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 主入口、命令、快捷键、托盘、窗口
│   │   ├── database.rs           # SQLite + FTS5 + 拼音
│   │   ├── tags.rs               # 规则标签
│   │   └── ai.rs                 # Ollama HTTP 客户端
│   ├── capabilities/
│   │   └── default.json          # 窗口权限
│   ├── icons/                    # 应用图标
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置
├── public/                       # 静态资源
├── docs/                         # 设计文档
├── releases/                     # 发布产物
│   ├── v1.0.0/
│   │   ├── ClipNest_1.0.0_x64_en-US.msi
│   │   ├── ClipNest_1.0.0_x64-setup.exe
│   │   └── SHA256SUMS.txt
│   └── v1.1.0/
│       ├── ClipNest_1.1.0_x64_en-US.msi
│       ├── ClipNest_1.1.0_x64-setup.exe
│       └── SHA256SUMS.txt
├── index.html                    # HTML 入口
├── package.json                  # Node 依赖
├── tsconfig.json                 # TS 配置
├── vite.config.ts                # Vite 构建
└── README.md                     # 本文件
```

### 添加新 Tauri 命令

1. 在 `src-tauri/src/lib.rs` 中实现：

```rust
#[tauri::command]
fn my_command(state: tauri::State<AppState>, arg: String) -> Result<String, String> {
    Ok(format!("got: {}", arg))
}
```

2. 注册到 `invoke_handler`：

```rust
.invoke_handler(tauri::generate_handler![
    my_command,
    // ...
])
```

3. 前端调用：

```typescript
const result = await invoke<string>("my_command", { arg: "hello" });
```

### 添加新窗口

1. 在 `lib.rs` 添加 `show_my_window` 函数（参考 `show_search_window`）
2. 在 `capabilities/default.json` 添加窗口名到 `windows` 数组
3. 前端按 `getCurrentWindow().label` 路由

### 数据库迁移

修改 `database.rs` 的 `initialize` 方法。已有用户数据库自动兼容（用 `let _ = ...` 静默处理已存在列）。

---

## 路线图

### v1.0 (current) ✅

- [x] 剪贴板捕获 (`Alt+W`)
- [x] 全文搜索 (`Alt+Space`)
- [x] 拼音搜索
- [x] 规则自动标签
- [x] AI 标签（Ollama 后端命令）
- [x] 三栏 ClipNest 风格 UI
- [x] 系统托盘（固定收藏 / 最近保存）
- [x] 永久本地存储（DELETE journal）
- [x] 失焦自动隐藏
- [x] 单实例运行
- [x] 强杀进程零数据丢失

### v1.1 (计划)

- [ ] 保存窗口 UI 集成 [+ AI] 按钮
- [ ] 详情面板标题/标签编辑（`update_clip` 命令）
- [ ] 开机自启设置
- [ ] 自定义全局快捷键
- [ ] Markdown 导出（已有 `export_markdown` 命令，待 UI 集成）

### v2.0 (愿景)

- [ ] 图片/文件剪贴板支持
- [ ] 向量搜索（sqlite-vec）
- [ ] 自然语言查询
- [ ] 多设备同步（可选 WebDAV）
- [ ] 标签管理页
- [ ] macOS / Linux 支持

---

## 许可证

MIT License — 自由使用、修改、分发。

---

## 反馈与贡献

- **问题反馈**: 通过 GitHub Issues
- **功能建议**: 通过 GitHub Discussions
- **代码贡献**: 欢迎 PR

---

<p align="center">
  <sub>用 ❤️ 和 Rust 制作</sub>
  <br/>
  <sub>v1.1.0 · 2026-06</sub>
</p>
