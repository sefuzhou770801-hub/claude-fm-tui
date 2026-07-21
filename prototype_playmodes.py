#!/usr/bin/env python3
# PROTOTYPE — throwaway。回答的问题见下方 QUESTION。
# 验证完请把 verdict 写回 issue/对话；本文件进 throwaway 分支，不进生产逻辑。
"""
QUESTION
--------
Claude FM 终端产品最终采用哪种「播放形态」？

  A 终端全屏位图（接近浏览器看直播）
  B 终端像素/色块（任意终端可跑）
  C 系统窗口高清 + 终端只当遥控器
  D 终端分屏（上画面 / 下控制条）

用户用数字键真机试播，用 * 标记偏好，q 退出并打印 verdict 草稿。
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import termios
import tty
from dataclasses import dataclass, field
from typing import Optional

URL = "https://www.youtube.com/live/tRsQsTMvPNg"
FORMAT = "96/95/94/best"  # 1080p → 720p → …

# ── ANSI ──────────────────────────────────────────────
BOLD = "\x1b[1m"
DIM = "\x1b[2m"
RESET = "\x1b[0m"
ORANGE = "\x1b[38;2;217;119;87m"
GREEN = "\x1b[32m"
CYAN = "\x1b[36m"
CLEAR = "\x1b[2J\x1b[H"


@dataclass
class Variant:
    key: str
    name: str
    tagline: str
    feel: str
    needs: str
    layout: str  # ASCII 示意
    mpv_args: list[str]


VARIANTS: list[Variant] = [
    Variant(
        key="A",
        name="终端全屏位图",
        tagline="像浏览器打开直播页，画面铺满本终端",
        feel="网页感最强 · 清晰度取决于终端像素与 kitty 协议",
        needs="Ghostty / WezTerm / Kitty（kitty 图形协议）",
        layout="""
┌─ 本终端 100% ─────────────────────┐
│                                   │
│     ████ 直播画面（位图）████      │
│     （mpv --vo=kitty 全屏）        │
│                                   │
│  q退出  空格暂停  9/0音量          │
└───────────────────────────────────┘""",
        mpv_args=[
            "--vo=kitty",
            "--vo-kitty-alt-screen=yes",
            "--vo-kitty-use-shm=yes",
            "--hwdec=no",
            "--force-window=no",
            "--keepaspect=yes",
            f"--ytdl-format={FORMAT}",
            "--title=Claude FM · prototype A",
            "--really-quiet",
            URL,
        ],
    ),
    Variant(
        key="B",
        name="终端像素/色块",
        tagline="任意真彩色终端可跑，故意复古、会糊",
        feel="像素风 · 不是网页高清，但是「全在终端里」",
        needs="任意 truecolor 终端（含系统 Terminal）",
        layout="""
┌─ 本终端 100% ─────────────────────┐
│ ░░▒▒▓▓██ 色块拼出的画面 ██▓▓▒▒░░ │
│ （mpv --vo=tct half-blocks）       │
│ 分辨率 ≈ 列×行×2，糊是正常的       │
│  q退出  空格暂停                   │
└───────────────────────────────────┘""",
        mpv_args=[
            "--vo=tct",
            "--vo-tct-algo=half-blocks",
            "--vo-tct-256=no",
            "--vo-tct-buffering=frame",
            "--hwdec=no",
            "--force-window=no",
            f"--ytdl-format={FORMAT}",
            "--title=Claude FM · prototype B",
            "--really-quiet",
            URL,
        ],
    ),
    Variant(
        key="C",
        name="窗口高清 + 终端遥控",
        tagline="画面在系统窗口（接近浏览器清晰度），终端只显示状态",
        feel="最清晰 · 但画面不在终端格子里（MovieBox 原路）",
        needs="任意终端 + mpv 窗口",
        layout="""
┌─ 终端（遥控器）─┐   ┌─ 系统窗口 ──────┐
│ ● Claude FM     │   │                │
│ ▶ playing 03:12 │   │  1080p 真画面   │
│ [q]停 [r]重连   │   │  （mpv gpu）    │
└─────────────────┘   └────────────────┘""",
        mpv_args=[
            "--vo=gpu-next",
            "--hwdec=auto",
            "--force-window=yes",
            "--geometry=70%",
            "--keepaspect=yes",
            f"--ytdl-format={FORMAT}",
            "--title=Claude FM · prototype C",
            "--really-quiet",
            URL,
        ],
    ),
    Variant(
        key="D",
        name="终端分屏",
        tagline="上半画面、下半状态条，同屏但不铺满",
        feel="能同时看状态 · 画面变小 · 实现脆弱",
        needs="kitty 协议终端；布局依赖行列数",
        layout="""
┌─ 本终端 ──────────────────────────┐
│ ▓▓▓▓▓ 上半：直播画面 ▓▓▓▓▓▓▓▓▓  │
│ ▓▓▓▓▓ （kitty 固定行区）▓▓▓▓▓▓▓  │
├───────────────────────────────────┤
│ ● Claude FM  playing · 03:12      │
│ [space]暂停  [q]退出              │
└───────────────────────────────────┘""",
        mpv_args=[
            "--vo=kitty",
            "--vo-kitty-alt-screen=no",
            "--vo-kitty-use-shm=yes",
            "--vo-kitty-top=1",
            "--vo-kitty-left=1",
            "--vo-kitty-rows=18",
            "--hwdec=no",
            "--force-window=no",
            f"--ytdl-format={FORMAT}",
            "--title=Claude FM · prototype D",
            "--really-quiet",
            URL,
        ],
    ),
]


@dataclass
class State:
    """暴露给用户看的完整 state（每帧打印）。"""

    screen: str = "menu"  # menu | detail | playing
    focus: str = "A"  # A|B|C|D
    last_played: Optional[str] = None
    last_exit: Optional[str] = None
    preferred: Optional[str] = None
    tries: dict = field(default_factory=dict)  # key -> count
    notes: str = ""

    def to_lines(self) -> list[str]:
        return [
            f"{BOLD}state{RESET}",
            f"  screen       {self.screen}",
            f"  focus        {self.focus}",
            f"  last_played  {self.last_played or '—'}",
            f"  last_exit    {self.last_exit or '—'}",
            f"  preferred    {GREEN}{self.preferred or '—（按 * 标记）'}{RESET}",
            f"  tries        {self.tries or '{}'}",
            f"  notes        {self.notes or '—'}",
        ]


def read_key() -> str:
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        ch = sys.stdin.read(1)
        if ch == "\x1b":
            # 可能是方向键
            rest = sys.stdin.read(2)
            if rest == "[A":
                return "up"
            if rest == "[B":
                return "down"
            if rest == "[C":
                return "right"
            if rest == "[D":
                return "left"
            return "esc"
        return ch
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)


def variant_by(key: str) -> Variant:
    for v in VARIANTS:
        if v.key == key:
            return v
    return VARIANTS[0]


def cycle_focus(cur: str, delta: int) -> str:
    keys = [v.key for v in VARIANTS]
    i = keys.index(cur)
    return keys[(i + delta) % len(keys)]


def render(state: State) -> None:
    sys.stdout.write(CLEAR)
    v = variant_by(state.focus)

    lines = [
        f"{ORANGE}{BOLD}● Claude FM · playmode prototype{RESET}",
        f"{DIM}问题：最终采用哪种播放形态？真机试播后按 * 标记偏好。{RESET}",
        "",
    ]

    if state.screen == "menu":
        lines.append(f"{BOLD}选一个形态（↑↓ 或 1–4，Enter 看详情，p 直接试播）{RESET}")
        lines.append("")
        for item in VARIANTS:
            mark = "▶" if item.key == state.focus else " "
            star = f" {GREEN}★ preferred{RESET}" if state.preferred == item.key else ""
            tried = f" {DIM}(试过 {state.tries.get(item.key, 0)} 次){RESET}" if item.key in state.tries else ""
            lines.append(
                f" {mark} {BOLD}{item.key}{RESET}  {item.name}{star}{tried}"
            )
            lines.append(f"     {DIM}{item.tagline}{RESET}")
        lines.append("")
        lines.append(f"{CYAN}当前焦点 · {v.key} {v.name}{RESET}")
        lines.append(f"{DIM}需要：{v.needs}{RESET}")
        lines.append(f"{DIM}手感：{v.feel}{RESET}")

    elif state.screen == "detail":
        lines.append(f"{BOLD}{v.key} — {v.name}{RESET}")
        lines.append(f"{v.tagline}")
        lines.append("")
        lines.append(f"{BOLD}布局示意{RESET}{v.layout}")
        lines.append("")
        lines.append(f"{BOLD}手感{RESET}  {v.feel}")
        lines.append(f"{BOLD}依赖{RESET}  {v.needs}")
        lines.append("")
        lines.append(f"{DIM}mpv 参数预览：{RESET}")
        # 不打印完整 URL 太长，只列 vo 相关
        preview = " ".join(a for a in v.mpv_args if a.startswith("--") and "ytdl" not in a)[:120]
        lines.append(f"  {DIM}{preview} …{RESET}")

    elif state.screen == "playing":
        lines.append(f"{ORANGE}正在启动形态 {v.key}…{RESET}")
        lines.append(f"{DIM}播完或按 q 退出 mpv 后会回到本菜单。{RESET}")

    lines.append("")
    lines.extend(state.to_lines())
    lines.append("")
    lines.append(
        f"{BOLD}keys{RESET}  "
        f"{DIM}[↑↓/jk] 切换  [1-4] 跳转  [Enter] 详情  [p] 试播  "
        f"[*] 标为偏好  [b] 回菜单  [q] 退出并打印 verdict{RESET}"
    )

    sys.stdout.write("\n".join(lines) + "\n")
    sys.stdout.flush()


def play(state: State) -> None:
    if not shutil.which("mpv"):
        state.last_exit = "error: mpv 未安装 (brew install mpv)"
        state.notes = "先装 mpv 再试播"
        state.screen = "menu"
        return

    v = variant_by(state.focus)
    state.screen = "playing"
    state.last_played = v.key
    state.tries[v.key] = state.tries.get(v.key, 0) + 1
    render(state)

    # 离开 raw/我们的 UI，把终端还给 mpv
    cmd = ["mpv", *v.mpv_args]
    try:
        # 恢复规范终端再跑 mpv
        proc = subprocess.run(cmd)
        state.last_exit = f"code={proc.returncode}"
        if proc.returncode != 0 and v.key in ("A", "D"):
            state.notes = (
                f"形态 {v.key} 失败常见原因：当前终端不支持 kitty。"
                "可改试 B（像素）或 C（窗口）。"
            )
    except Exception as e:
        state.last_exit = f"error: {e}"
    state.screen = "menu"


def print_verdict(state: State) -> None:
    print()
    print(f"{ORANGE}{BOLD}── prototype verdict 草稿 ──{RESET}")
    print(f"preferred: {state.preferred or '(未标记)'}")
    print(f"tries:     {state.tries}")
    print(f"last:      {state.last_played} exit={state.last_exit}")
    print()
    print("建议下一步（按 preferred）：")
    if state.preferred == "A":
        print("  → 定稿：claudefm 默认 kitty 全屏；无协议时提示 --gui / --tct")
    elif state.preferred == "B":
        print("  → 定稿：默认 tct 像素风；接受糊，强调「全在终端」")
    elif state.preferred == "C":
        print("  → 定稿：MovieBox 路线 — 终端状态条 + mpv 窗口 1080p")
    elif state.preferred == "D":
        print("  → 定稿：分屏；实现成本高，需固定 TUI 行数与 resize 处理")
    else:
        print("  → 再跑一遍 python3 prototype_playmodes.py，试播后按 *")
    print()
    print(f"{DIM}本文件是 throwaway prototype，不要当生产代码合并。{RESET}")


def main() -> int:
    if not sys.stdin.isatty():
        print("请在真实终端运行：python3 prototype_playmodes.py", file=sys.stderr)
        return 1

    state = State()
    render(state)

    while True:
        key = read_key()

        if key in ("q", "\x03"):
            render(state)
            # 退出 raw 后打印
            print_verdict(state)
            return 0

        if key in ("b", "esc") and state.screen != "menu":
            state.screen = "menu"
            render(state)
            continue

        if key in ("1", "2", "3", "4"):
            state.focus = "ABCD"[int(key) - 1]
            state.screen = "menu"
            render(state)
            continue

        if key in ("up", "k", "left"):
            state.focus = cycle_focus(state.focus, -1)
            render(state)
            continue

        if key in ("down", "j", "right"):
            state.focus = cycle_focus(state.focus, 1)
            render(state)
            continue

        if key in ("\r", "\n"):
            state.screen = "detail"
            render(state)
            continue

        if key == "p" or key == " ":
            # 试播会占用终端；返回后重画
            play(state)
            render(state)
            continue

        if key == "*":
            state.preferred = state.focus
            state.notes = f"用户标记 {state.focus} 为 preferred"
            render(state)
            continue

        # 未知键：重画，不炸
        render(state)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print()
        raise SystemExit(0)
