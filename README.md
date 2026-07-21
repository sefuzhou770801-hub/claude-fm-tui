# Claude FM TUI

从 [MovieBox-Tui](https://github.com/mesamirh/MovieBox-Tui) fork 改造：只做一件事——在终端收听 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg) 直播。

Claude FM 是 Anthropic 的 24 小时直播电台，定位 *music for thinking and building*。

## 依赖

任选一条播放链路：

| 优先级 | 需要 | 说明 |
|--------|------|------|
| 1 | [`mpv`](https://mpv.io)（自带 yt-dlp 支持） | `brew install mpv` |
| 2 | `yt-dlp` + `ffplay`（ffmpeg） | macOS 通常已有 ffmpeg；再装 `brew install yt-dlp` |

## 安装与运行

```sh
cargo install --path .
claude-fm
```

或直接：

```sh
cargo run --release
```

## 操作

| 按键 | 作用 |
|------|------|
| `Enter` / `Space` | 播放 / 停止 |
| `r` | 重新连接直播 |
| `q` / `Esc` | 退出 |

启动后若检测到播放器，会自动开始播放 Claude FM。

## 固定源

```
https://www.youtube.com/live/tRsQsTMvPNg
```

本项目不搜索、不选片，只播这一路直播。

## 许可

沿用上游：MIT OR Apache-2.0。
