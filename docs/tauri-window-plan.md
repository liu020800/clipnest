# ClipNest Tauri Window Plan

## Capture Window

- `transparent: true`
- `decorations: false`
- `alwaysOnTop: true`
- `resizable: false`
- `width: 540`
- `height: 390`
- `center: true`
- `skipTaskbar: true`
- global shortcut: `Ctrl + Shift + V`

## Library Main Window

- `width: 1120`
- `height: 720`
- `minWidth: 960`
- `minHeight: 620`
- `decorations: false` or custom titlebar

## Tray

- Left click: open main window
- Right click menu:
  - Quick capture
  - Open library
  - Pause clipboard listener
  - Settings
  - Quit

## Future Rust Work

- Clipboard listener
- Global shortcuts
- SQLite storage
- Local file export
- AI API calls
