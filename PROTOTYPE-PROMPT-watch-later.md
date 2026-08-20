# PROTOTYPE PROMPT · YouTube/X 链接 → 终端待播清单

> 复制下面「PROMPT 正文」整段，丢给 Claude Code / Cursor / Codex 即可开写原型。  
> 范围：throwaway 可跑通闭环，不是产品抛光。  
> 基石仓库：`~/projects/claude-fm-tui`（MovieBox-Tui fork，bin=`claudefm`，mpv + kitty 位图，形态 D 定稿）。

---

## PROMPT 正文（从下一行开始复制）

```text
你在 macOS 上做一个「本地待播」最小可跑原型。目标不是完整产品，是验证这条闭环：

  我（用户）把 YouTube / X 视频链接发给 agent
  → agent 在本机入队（下载 + 中字）
  → 我在终端打开待播清单，选一条就能播，并自动挂中文字幕

# 背景与边界

- 播放器基石已有：`/Users/zhousefu/projects/claude-fm-tui`
  - Rust，bin 名 `claudefm`
  - 依赖：`mpv`（brew）
  - 定稿播放形态 D：终端上半画面（kitty 图形协议）+ 底栏仅 1 行状态
  - 现况：硬编码 Claude FM 直播 URL，是单用途播放器，不是通用片单
  - 可复用：mpv spawn、IPC 按键、分屏状态栏、`--gui` / `--tct` 兜底
  - 要改：数据源、清单、本地文件、字幕
- 研究素材目录（已有下载/转写惯例，可参考，不要强绑产品路径）：
  `/Users/zhousefu/Downloads/x-videos/`
  惯例：yt-dlp → ffmpeg 16k mono → faster-whisper base → FULL.transcript(.plain).txt
- 明确不做：
  - 不要做成 LifeOS / OpenMy 功能模块
  - 不要云同步、不要账号、不要 Web UI
  - 不要完美 TUI 动画；清单第一版可以用数字键选片
  - 不要改坏现有 `claudefm` 直播模式（可加新 bin 或子命令）

# 产品形态（两屏）

1) 清单屏（待播）
   ○ Elvis · Multimodal…   9m  [中字✓]
   ● Kun agentic…         62m  [中字✓]  ← 光标
   ○ Wes second brain…    37m  [转写中…]

2) 播放屏（沿用形态 D）
   [视频区域]
   ● 标题  ▶ 12:04/62:00  中字  [q]退出 [s]切字幕 [n]下一条

# 目录约定（入队与播放共用，先写死路径）

根目录：`~/Movies/watch-later/`

每条视频一个目录：
~/Movies/watch-later/<id>/
  video.mp4          # 必需才能进「可播」
  zh.srt             # 中字；没有也可以进清单，标记 [无中字]
  en.srt             # 可选
  meta.json          # 必需
  SUMMARY.md         # 可选，原型可忽略
  audio.wav          # 中间产物，可删可留

<meta.json> 最小字段：
{
  "id": "string",
  "title": "string",
  "source_url": "https://...",
  "duration_sec": 0,
  "status": "queued|downloading|transcribing|ready|error",
  "error": null,
  "created_at": "ISO-8601",
  "updated_at": "ISO-8601",
  "has_video": true,
  "has_zh_srt": true
}

id 规则：从 URL 稳定派生（YouTube video id / X status id），避免重复入队。

# 两段职责（严格拆开）

## A. 入队管线（脚本，agent 调用）

路径建议：`~/projects/claude-fm-tui/scripts/enqueue.sh`
（也可用 Python，但对外只暴露一条命令）

用法：
  ./scripts/enqueue.sh "<url>"

行为：
1. 解析 URL，得到 id；若 `~/Movies/watch-later/<id>/` 已存在且 status=ready，直接打印路径并退出 0（幂等）
2. 创建目录，写 meta.json（status=downloading）
3. yt-dlp 下载到 `video.mp4`（选合理画质，优先 mp4；失败写 status=error）
4. 用 ffmpeg 抽 16k mono wav（或直接喂 whisper 支持的格式）
5. faster-whisper base（或本机已有 whisper）出英/原语言 srt；再尽量产出 `zh.srt`
   - 若已有中文音轨/软字幕可优先抽取
   - 机器翻译中字可先用最简单可用方案；没有稳定翻译时：至少产出原语言 srt，并在 meta 标明 has_zh_srt=false
6. 更新 meta：duration、title、status=ready / error
7. stdout 打印一行人类可读结果：
   READY <id> <title> -> ~/Movies/watch-later/<id>

依赖允许：yt-dlp、ffmpeg、faster-whisper/whisper、python3。缺依赖时给出 brew/pip 安装提示并非零退出。

## B. 终端播放器（基于 claude-fm-tui）

在现有仓库扩展，不要另起新仓。

优先实现顺序：
1. 通用本地播放：`spawn_mpv` 支持本地 `video.mp4` + 可选 `--sub-file=zh.srt` + `--save-position-on-quit`
2. 扫描 `~/Movies/watch-later/*/meta.json`，列清单
3. 清单交互（第一版即可）：
   - 启动新 bin 或子命令：`watchq`（推荐）或 `claudefm queue`
   - 显示标题 / 时长 / 字幕状态
   - ↑↓ 或 j/k 移动，Enter 播放；或直接输入序号
   - 只把 status=ready 且 has_video=true 的标为可播；其它显示状态但不播
4. 播放中键位：
   - q 退出回清单（或退出程序，先定一种并写进 --help）
   - 空格 暂停
   - 9/0 音量，m 静音（可复用现逻辑）
   - n 下一条（有则切）
   - s 切换字幕 on/off（有 zh.srt 时）
5. 底栏 1 行：标题 · 进度 · 字幕状态（不要再写死 Claude FM live）
6. 保留旧直播模式：`claudefm` 默认行为可不变；新路径走 `watchq`

终端假设：Ghostty / WezTerm / Kitty。无 kitty 协议时 `--gui` 兜底仍可用。

# 验收（必须亲手跑通并报告）

1. 准备样例：用一条短 YouTube 或已有本地 mp4 伪造一个 `~/Movies/watch-later/demo/`（含 meta + video；zh.srt 可先手写两行）
2. `watchq`（或你实现的命令）能列出 demo
3. Enter 后终端里出画 + 底栏；有 srt 则字幕可见
4. `scripts/enqueue.sh "<一条真实短视频 URL>"` 能产出 ready 目录（若网络/模型失败，说明失败点与最小替代路径）
5. README 补 10 行以内：安装依赖、入队命令、打开清单命令

# 非目标（写了也算 scope creep）

- 好看的 ratatui 多栏、搜索、标签、多队列
- 后台 daemon / launchd
- Telegram bot 本体（入队由外部 agent 调脚本即可）
- 自动写 SUMMARY / 知识库入库

# 交付物

- 可运行代码（本仓分支即可）
- `scripts/enqueue.sh`（或等价）
- 更新后的简短 README 用法
- 终端里跑通的命令记录（复制你实际执行的命令与结果）

先做最小闭环，再考虑美化。开始前用 5 行说明你的实现切片，然后直接改代码。
```

---

## 使用方式

1. 打开 Claude Code / Cursor，workdir：`~/projects/claude-fm-tui`
2. 粘贴「PROMPT 正文」
3. 原型验收通过后，再把「Telegram 发链接 → Hermes 调 enqueue」接上（那是第二阶段，不必写进第一版原型）

## 第二阶段（先别写进原型）

- Hermes 收到 YouTube/X 链接 → 调 `enqueue.sh` → 回用户「已入队 / ready」
- 用户本机终端自己跑 `watchq` 看片
- 可选：入队完成后 macOS 通知
