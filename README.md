# ClipNest

Windows 剪贴板历史与搜索工具。复制文本后，按 `Ctrl+Alt+Space` 打开搜索。默认列表先显示置顶内容，再按最近复制或使用的时间排列。用方向键选择条目，按 `Enter` 粘贴回原窗口。

## 安装与使用

从 [GitHub Releases](https://github.com/liu020800/clipnest/releases/latest) 下载 Windows x64 安装包，EXE 和 MSI 任选其一。

| 默认快捷键 | 操作 |
| --- | --- |
| `Ctrl+Alt+Space` | 搜索剪贴板历史和已保存条目 |
| `Ctrl+Shift+S` | 保存当前剪贴板内容，可编辑标题和标签 |
| `Ctrl+Shift+O` | 框选屏幕文字并进行 OCR |
| `Esc` | 关闭搜索窗口 |

历史与设置保存在 `%APPDATA%\com.copyliusq.desktop\copyliusq.db`。可在设置中备份或导出数据。AI 标签默认关闭，启用后会将待标记文本发送给所配置的服务。

## 从源码构建

Windows x64 需要 Node.js 22 LTS、Rust MSVC 工具链、Visual Studio C++ 构建工具、WebView2 和 .NET 8 SDK。在项目根目录运行以下 PowerShell 命令。

```powershell
npm ci
dotnet build scripts/ocr_host/ClipNest.OcrHost.csproj -c Release
npm run tauri -- build
```

安装包生成在 `src-tauri/target/release/bundle/`。开发模式使用 `npm run tauri -- dev`。

仅供个人使用。
