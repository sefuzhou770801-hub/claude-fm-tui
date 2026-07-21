# Claude FM（终端内播放）

从 [MovieBox-Tui](https://github.com/mesamirh/MovieBox-Tui) fork 改造：只做一件事——在**当前终端窗口里**播放 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg) 直播（有声有画）。

## 原理

用 `mpv` 的终端视频输出，把画面直接画进你正在用的终端：

| 后端 | 适用终端 | 画质 |
|------|----------|------|
| `kitty` | Ghostty、WezTerm、Kitty | 高（图形协议） |
| `tct` | iTerm2、Terminal.app、多数真彩色终端 | 中（彩色字符块） |

声音走系统音频，画面留在终端。

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

## 强制指定渲染后端

```sh
CLAUDEFM_VO=kitty claudefm
CLAUDEFM_VO=tct claudefm
```

若某终端里画面花屏/空白，换另一个后端试试。

## 固定源

```
https://www.youtube.com/live/tRsQsTMvPNg
```

## 许可

沿用上游：MIT OR Apache-2.0。
