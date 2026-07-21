# Claude FM（终端内播放）

从 [MovieBox-Tui](https://github.com/mesamirh/MovieBox-Tui) fork 改造：只做一件事——在**当前终端窗口里**播放 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg) 直播（有声有画）。

## 原理与清晰度

用 `mpv` 把画面画进终端（或独立窗口）：

| 模式 | 命令 | 画质 | 说明 |
|------|------|------|------|
| `kitty` | `claudefm --kitty`（默认优先） | **高** | 终端内真正位图，需 Ghostty / WezTerm / Kitty 等 |
| `tct` | `claudefm --tct` | **低** | 每个字符一格色块，糊是物理限制，不是源糊 |
| `gui` | `claudefm --gui` | **最高** | 独立 mpv 窗口，接近源站 1080p |

直播源本身有 144p～1080p；默认优先拉 **1080p**。  
若你觉得糊，先看启动时打印的 `vo=`：若是 `tct`，换现代终端或 `--gui`。

## 依赖

```sh
brew install mpv   # 自带 yt-dlp 支持
```

## 安装与运行

```sh
cargo install --path .
claudefm
```

## 操作（mpv 快捷键）

| 按键 | 作用 |
|------|------|
| `q` | 退出 |
| `f` | 全屏（终端内） |
| `9` / `0` | 音量减 / 加 |
| `m` | 静音 |
| `SPACE` | 暂停 |

## 指定渲染后端

```sh
claudefm --kitty          # 终端内高清（推荐）
claudefm --tct            # 兼容模式（会糊）
claudefm --gui            # 独立窗口 1080p
CLAUDEFM_VO=gui claudefm  # 环境变量写法
```

花屏/空白时依次试：`--kitty` → `--tct` → `--gui`。

## 固定源

```
https://www.youtube.com/live/tRsQsTMvPNg
```

## 许可

沿用上游：MIT OR Apache-2.0。
