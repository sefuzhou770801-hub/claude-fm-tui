# Claude FM

终端里播 [Claude FM](https://www.youtube.com/live/tRsQsTMvPNg) 直播。

**定稿形态 D（已收窄）：**

```
┌──────────────────────────────┐
│                              │
│     直播画面（kitty 位图）      │  ← 几乎全屏
│                              │
├──────────────────────────────┤
│ ● Claude FM  ▶ live  03:12   │  ← 仅 1 行状态栏
└──────────────────────────────┘
```

## 依赖

```sh
brew install mpv
```

默认分屏需要 **Ghostty / WezTerm / Kitty**（kitty 图形协议）。

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
| `m` | 静音 |

### 兜底

```sh
claudefm --gui   # 系统窗口 1080p
claudefm --tct   # 全屏像素色块
```

## 原型

播放形态对比（已决策 D + 单行底栏）：

```sh
python3 prototype_playmodes.py
```

## 许可

MIT OR Apache-2.0。
