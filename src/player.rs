//! 终端内播放：mpv 用 kitty / tct 把画面画在当前终端，声音走系统音频。

use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Claude FM 直播地址
pub const CLAUDE_FM_URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";

/// 终端视频输出后端
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalVo {
    /// Kitty 图形协议（Ghostty / WezTerm / Kitty 画质最好）
    Kitty,
    /// True-color 字符块（通用，多数终端可用）
    Tct,
}

impl TerminalVo {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kitty => "kitty",
            Self::Tct => "tct",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Kitty => "kitty 图形协议（终端内高清）",
            Self::Tct => "tct 真彩色字符（通用终端）",
        }
    }

    /// 根据当前终端环境选 VO
    pub fn detect() -> Self {
        // 显式覆盖：CLAUDEFM_VO=kitty|tct
        if let Ok(v) = env::var("CLAUDEFM_VO") {
            match v.to_ascii_lowercase().as_str() {
                "kitty" => return Self::Kitty,
                "tct" | "ansi" | "ascii" => return Self::Tct,
                _ => {}
            }
        }

        if env::var_os("KITTY_WINDOW_ID").is_some() {
            return Self::Kitty;
        }

        let program = env::var("TERM_PROGRAM").unwrap_or_default();
        let term = env::var("TERM").unwrap_or_default().to_ascii_lowercase();

        // Ghostty / WezTerm / Kitty 支持 kitty 协议，效果明显好于 tct
        match program.as_str() {
            "ghostty" | "WezTerm" | "kitty" => return Self::Kitty,
            // iTerm2 对 kitty 协议支持不完整，走 tct 更稳
            "iTerm.app" | "Apple_Terminal" | "Orca" => return Self::Tct,
            _ => {}
        }

        if term.contains("kitty") || term.contains("ghostty") || term.contains("wezterm") {
            return Self::Kitty;
        }

        Self::Tct
    }
}

pub fn mpv_available() -> bool {
    command_exists("mpv")
}

/// 在当前终端前台播放（阻塞直到 mpv 退出）。画面直接画在本终端里。
pub fn play_in_terminal(url: &str, vo: TerminalVo) -> io::Result<i32> {
    if !mpv_available() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "未找到 mpv。请先执行：brew install mpv",
        ));
    }

    let mut cmd = Command::new("mpv");
    cmd.arg(format!("--vo={}", vo.as_str()))
        // 终端 VO 必须走软件渲染，避免抢 GPU 窗口
        .arg("--hwdec=no")
        .arg("--gpu-context=auto")
        // 不要弹独立 GUI 窗口
        .arg("--force-window=no")
        .arg("--keep-open=no")
        .arg("--ytdl-format=bestvideo+bestaudio/best")
        .arg("--title=Claude FM")
        // 少刷状态行，避免干扰画面；错误仍可见
        .arg("--msg-level=all=error,ytdl_hook=status")
        .arg("--really-quiet")
        .arg(url);

    // tct 专用：半块字符，密度更高
    if vo == TerminalVo::Tct {
        cmd.arg("--vo-tct-algo=half-blocks");
        cmd.arg("--vo-tct-256=yes");
    }

    // 继承当前终端：stdin 给按键，stdout/stderr 画画面
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}

pub fn print_banner(vo: TerminalVo) {
    let mut out = io::stderr();
    let _ = writeln!(
        out,
        "\x1b[1;35mClaude FM\x1b[0m  ·  终端内播放  ·  vo={}\n\
         \x1b[2m{}\x1b[0m\n\
         \x1b[2m源 {}\x1b[0m\n\
         \x1b[33m[q]\x1b[0m 退出  \x1b[33m[f]\x1b[0m 全屏  \x1b[33m[9/0]\x1b[0m 音量  \x1b[33m[m]\x1b[0m 静音\n\
         \x1b[2m强制后端：CLAUDEFM_VO=kitty|tct\x1b[0m\n",
        vo.as_str(),
        vo.label(),
        CLAUDE_FM_URL
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
