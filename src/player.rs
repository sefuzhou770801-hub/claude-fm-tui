//! 终端内播放：mpv 用 kitty / tct 把画面画在当前终端；也支持 --gui 高清窗口。

use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Claude FM 直播地址
pub const CLAUDE_FM_URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";

/// 直播清晰度：优先 1080p / 720p（源站 m3u8 合并流，不是 DASH 分离轨）
const YTDL_FORMAT: &str = "96/95/94/best";

/// 视频输出方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    /// Kitty 图形协议 — 终端内真正位图，清晰度最高（终端内）
    Kitty,
    /// True-color 字符块 — 一格一个色块，必然糊
    Tct,
    /// 系统原生窗口 — 接近完整 1080p
    Gui,
}

impl VideoMode {
    pub fn as_vo(self) -> &'static str {
        match self {
            Self::Kitty => "kitty",
            Self::Tct => "tct",
            Self::Gui => "gpu-next",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Kitty => "kitty 位图协议（终端内高清）",
            Self::Tct => "tct 字符色块（终端内，清晰度受格子限制，会明显发糊）",
            Self::Gui => "独立窗口 gpu（接近源站 1080p）",
        }
    }

    /// 解析 CLI / 环境变量，再自动探测
    pub fn resolve(args: &[String]) -> Self {
        // 1) 命令行优先
        for a in args {
            match a.as_str() {
                "--gui" | "-g" | "--window" => return Self::Gui,
                "--kitty" | "-k" => return Self::Kitty,
                "--tct" | "-t" | "--ansi" => return Self::Tct,
                _ => {}
            }
        }

        // 2) 环境变量
        if let Ok(v) = env::var("CLAUDEFM_VO") {
            match v.to_ascii_lowercase().as_str() {
                "gui" | "gpu" | "window" | "mpv" => return Self::Gui,
                "kitty" => return Self::Kitty,
                "tct" | "ansi" | "ascii" => return Self::Tct,
                _ => {}
            }
        }

        // 3) 自动探测：优先 kitty（画质远好于 tct）
        Self::detect_terminal()
    }

    fn detect_terminal() -> Self {
        if env::var_os("KITTY_WINDOW_ID").is_some() {
            return Self::Kitty;
        }

        let program = env::var("TERM_PROGRAM").unwrap_or_default();
        let term = env::var("TERM").unwrap_or_default().to_ascii_lowercase();

        // 仅系统自带 Terminal 几乎只有 tct 可用
        if program == "Apple_Terminal" {
            return Self::Tct;
        }

        // Ghostty / WezTerm / Kitty / 多数现代终端：kitty 协议
        if matches!(
            program.as_str(),
            "ghostty" | "WezTerm" | "kitty" | "iTerm.app" | "Orca" | "WarpTerminal"
        ) {
            // iTerm2 新版本对 kitty 协议有支持；不行用户可 CLAUDEFM_VO=tct
            return Self::Kitty;
        }

        if term.contains("kitty")
            || term.contains("ghostty")
            || term.contains("wezterm")
            || term.contains("iterm")
        {
            return Self::Kitty;
        }

        // 未知终端也先试 kitty：失败再让用户切 tct/gui
        // 真彩色老终端再退 tct
        if env::var("COLORTERM").ok().as_deref() == Some("truecolor")
            || env::var("COLORTERM").ok().as_deref() == Some("24bit")
        {
            return Self::Kitty;
        }

        Self::Kitty
    }
}

pub fn mpv_available() -> bool {
    command_exists("mpv")
}

/// 在当前终端（或 GUI 窗口）前台播放，阻塞直到退出。
pub fn play(url: &str, mode: VideoMode) -> io::Result<i32> {
    if !mpv_available() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "未找到 mpv。请先执行：brew install mpv",
        ));
    }

    let mut cmd = Command::new("mpv");

    match mode {
        VideoMode::Gui => {
            cmd.arg("--vo=gpu-next")
                .arg("--hwdec=auto")
                .arg("--force-window=yes")
                .arg("--geometry=80%")
                .arg("--keepaspect=yes");
        }
        VideoMode::Kitty => {
            cmd.arg("--vo=kitty")
                .arg("--hwdec=no")
                .arg("--force-window=no")
                // 共享内存传输，帧更稳、更清晰
                .arg("--vo-kitty-use-shm=yes")
                .arg("--vo-kitty-alt-screen=yes")
                // 按终端像素缩放，避免被压成糊块
                .arg("--keepaspect=yes")
                .arg("--video-unscaled=no");
        }
        VideoMode::Tct => {
            cmd.arg("--vo=tct")
                .arg("--hwdec=no")
                .arg("--force-window=no")
                .arg("--vo-tct-algo=half-blocks")
                // 真 24bit，不要压成 256 色
                .arg("--vo-tct-256=no")
                .arg("--vo-tct-buffering=frame")
                .arg("--keepaspect=yes");
        }
    }

    cmd.arg(format!("--ytdl-format={YTDL_FORMAT}"))
        .arg("--title=Claude FM")
        .arg("--keep-open=no")
        // 终端模式下少打日志，避免冲掉画面
        .arg("--msg-level=all=error,ytdl_hook=status")
        .arg(url);

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}

pub fn print_banner(mode: VideoMode) {
    let mut out = io::stderr();
    let quality_hint = match mode {
        VideoMode::Kitty => "终端内位图，一般可接近源清晰度（放大终端窗口更佳）",
        VideoMode::Tct => {
            "字符色块模式：每个字符一格像素，看起来糊是正常的。\
             要高清请换 Ghostty/Kitty/WezTerm，或：claudefm --gui"
        }
        VideoMode::Gui => "独立播放窗口，源站最高约 1080p",
    };

    let _ = writeln!(
        out,
        "\x1b[1;35mClaude FM\x1b[0m  ·  {}  ·  vo={}\n\
         \x1b[2m{}\x1b[0m\n\
         \x1b[2m流：优先 1080p (96) → 720p (95) → 更低\x1b[0m\n\
         \x1b[33m[q]\x1b[0m 退出  \x1b[33m[9/0]\x1b[0m 音量  \x1b[33m[m]\x1b[0m 静音\n\
         \x1b[2m模式：claudefm --kitty | --tct | --gui\x1b[0m\n",
        quality_hint,
        mode.as_vo(),
        mode.label(),
    );
}

pub fn print_help() {
    eprintln!(
        "\x1b[1;35mclaudefm\x1b[0m — 终端播放 Claude FM\n\n\
         用法：\n\
           claudefm           自动选择（优先 kitty 高清）\n\
           claudefm --kitty   终端内位图（推荐，需 Ghostty/WezTerm/Kitty 等）\n\
           claudefm --tct     终端内字符色块（通用，但会糊）\n\
           claudefm --gui     独立窗口高清（接近 1080p）\n\n\
         环境变量：CLAUDEFM_VO=kitty|tct|gui\n\n\
         为何会糊？\n\
           tct 用字符当像素，分辨率 ≈ 终端列数×行数×2，无法和真视频比。\n\
           要清晰：用支持 kitty 协议的终端，或 claudefm --gui。\n"
    );
}

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
