# PROTOTYPE · Claude FM 播放形态

> **throwaway** — 用来回答一个问题，不是产品代码。

## 问题

Claude FM 最终用哪种播放形态？

| 键 | 形态 | 一句话 |
|----|------|--------|
| **A** | 终端全屏位图 | 最接近浏览器看直播 |
| **B** | 终端像素/色块 | 全在终端，但会糊 |
| **C** | 窗口高清 + 终端遥控 | 最清晰，画面在系统窗口 |
| **D** | 终端分屏 | 上画面下状态，实现脆 |

## 运行（一条命令）

```sh
cd ~/projects/claude-fm-tui
python3 prototype_playmodes.py
```

依赖：`mpv`（`brew install mpv`）。  
A/D 建议在 **Ghostty / WezTerm / Kitty** 里跑。

## 操作

| 键 | 作用 |
|----|------|
| `↑↓` / `j k` | 切换形态 |
| `1`–`4` | 直接跳到 A–D |
| `Enter` | 看布局示意 |
| `p` / `空格` | **真机试播**（起 mpv） |
| `*` | 标为当前偏好 |
| `b` | 回菜单 |
| `q` | 退出并打印 verdict 草稿 |

每帧底部会打印完整 `state`（screen / focus / tries / preferred）。

## 选完之后

把 `preferred` 回给对话或写进任务；实现只收 winner，本 prototype 留 throwaway 分支。

## Verdict（2026-07-21）

**preferred: D**，底栏只要 **1 行**状态。已折进 `claudefm` 默认行为。
