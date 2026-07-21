# Claude FM

在终端里全屏播放 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg)，效果接近浏览器打开直播页。

## 难不难？

不难。核心就一句：

```sh
mpv --vo=kitty --ytdl-format=96/95/94/best 'https://www.youtube.com/live/tRsQsTMvPNg'
```

`claudefm` 只是把上面这件事固定好、装成一个命令。

## 原理

| 方式 | 观感 |
|------|------|
| `kitty` 图形协议（默认） | 终端内真正位图，铺满窗口，接近浏览器 |
| `--gui` | 系统播放窗口，清晰度最高 |
| `--tct` | 字符色块，能播但糊 |

源站最高 1080p；默认优先拉 1080p。

## 依赖

```sh
brew install mpv
```

默认终端内高清需要支持 **kitty 图形协议** 的终端：

- Ghostty
- WezTerm
- Kitty

系统自带 Terminal.app 不行，请用上面之一，或 `claudefm --gui`。

## 使用

```sh
cargo install --path .
claudefm
```

| 键 | 作用 |
|----|------|
| `q` | 退出 |
| `空格` | 暂停 / 继续 |
| `9` / `0` | 音量 |
| `f` | 全屏 |

## 许可

MIT OR Apache-2.0。
