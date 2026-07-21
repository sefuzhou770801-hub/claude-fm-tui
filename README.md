# Claude FM

从 [MovieBox-Tui](https://github.com/mesamirh/MovieBox-Tui) fork 改造：在终端里收听/收看 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg)，**控制面板与画面同屏**，面板为 Claude Code 风格。

## 布局

```
┌──────────────────────────────────────────┐
│                                          │
│         直播画面（kitty 位图）              │  ← 上半
│                                          │
├──────────────────────────────────────────┤
│  ● Claude FM  /split                     │
│  status · playing · 波形                 │  ← 下半 TUI
│  session · source · note                 │     Claude Code 暖橙风格
│  [space] 播放/暂停  [g] 窗口  [q] 退出     │
└──────────────────────────────────────────┘
```

## 依赖

```sh
brew install mpv
```

分屏画面需要终端支持 **kitty 图形协议**（Ghostty、WezTerm、Kitty 等）。  
不支持时用窗口模式：`claudefm --gui`。

## 安装

```sh
cargo install --path .
claudefm
```

## 模式

| 命令 | 效果 |
|------|------|
| `claudefm` | 上半画面 + 下半 TUI（默认） |
| `claudefm --gui` | 独立 1080p 窗口 + TUI |
| `claudefm --audio` | 仅音频 + TUI |

## 快捷键

| 键 | 作用 |
|----|------|
| `space` / `Enter` | 播放 / 暂停 |
| `s` | 停止 |
| `r` | 重连 |
| `g` | 切到独立窗口 |
| `k` | 切回终端分屏 |
| `q` | 退出 |

## 许可

MIT OR Apache-2.0。
