//! Claude FM — 在终端里全屏播放直播，用法接近浏览器里打开直播页。
//!
//! 原理很简单：把终端交给 mpv，用图形协议把每一帧画满整个终端窗口。
//! 不搞分屏 TUI、不弹独立播放器窗口（除非你显式要求）。

use std::env;
use std::io::{self, Write};
use std::process::{Command, ExitCode, Stdio};

const URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";
/// 直播 m3u8：优先 1080p → 720p → 更低
const FORMAT: &str = "96/95/94/best";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        help();
        return ExitCode::SUCCESS;
    }

    if which("mpv").is_err() {
        eprintln!("需要 mpv。安装：brew install mpv");
        return ExitCode::FAILURE;
    }

    // --gui：真·系统窗口（和浏览器窗口类似，但不在终端格子里）
    let gui = args.iter().any(|a| a == "--gui" || a == "-g");
    let force_tct = args.iter().any(|a| a == "--tct");
    let force_kitty = args.iter().any(|a| a == "--kitty");

    let vo = if gui {
        Vo::Gui
    } else if force_tct {
        Vo::Tct
    } else if force_kitty {
        Vo::Kitty
    } else {
        Vo::detect()
    };

    if let Err(e) = play(vo) {
        eprintln!("播放失败：{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[derive(Clone, Copy)]
enum Vo {
    /// 终端内位图（接近浏览器观感，需 Ghostty/WezTerm/Kitty 等）
    Kitty,
    /// 终端内色块（能播，但会糊）
    Tct,
    /// 系统窗口
    Gui,
}

impl Vo {
    fn detect() -> Self {
        if let Ok(v) = env::var("CLAUDEFM_VO") {
            match v.to_ascii_lowercase().as_str() {
                "gui" | "gpu" | "window" => return Self::Gui,
                "tct" | "ansi" => return Self::Tct,
                "kitty" => return Self::Kitty,
                _ => {}
            }
        }
        // 几乎所有现代终端都优先试 kitty；不行用户再 --tct / --gui
        let program = env::var("TERM_PROGRAM").unwrap_or_default();
        if program == "Apple_Terminal" {
            // 系统终端不支持 kitty 协议
            return Self::Tct;
        }
        Self::Kitty
    }

}

fn play(vo: Vo) -> io::Result<()> {
    let mut stderr = io::stderr();
    let _ = writeln!(
        stderr,
        "\x1b[38;2;217;119;87mClaude FM\x1b[0m  ·  终端全屏播放  ·  {}\n\
         \x1b[2m加载直播后画面会铺满本终端。  q 退出  空格 暂停  9/0 音量\x1b[0m\n",
        match vo {
            Vo::Kitty => "kitty 位图（接近浏览器）",
            Vo::Tct => "tct 色块（会糊；建议 Ghostty/WezTerm 或 --gui）",
            Vo::Gui => "系统窗口",
        }
    );
    let _ = stderr.flush();

    let mut cmd = Command::new("mpv");
    cmd.arg(format!("--ytdl-format={FORMAT}"))
        .arg("--title=Claude FM")
        .arg("--keep-open=no")
        .arg(URL);

    match vo {
        Vo::Kitty => {
            cmd.args([
                "--vo=kitty",
                "--vo-kitty-alt-screen=yes", // 占满整个终端，像浏览器页
                "--vo-kitty-use-shm=yes",
                "--hwdec=no",
                "--force-window=no",
                "--keepaspect=yes",
            ]);
        }
        Vo::Tct => {
            cmd.args([
                "--vo=tct",
                "--vo-tct-algo=half-blocks",
                "--vo-tct-256=no",
                "--vo-tct-buffering=frame",
                "--hwdec=no",
                "--force-window=no",
            ]);
        }
        Vo::Gui => {
            cmd.args([
                "--vo=gpu-next",
                "--hwdec=auto",
                "--force-window=yes",
                "--geometry=80%",
                "--keepaspect=yes",
            ]);
        }
    }

    // 前台占用本终端：stdin 按键、stdout 画画面
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        // kitty 失败时给一条可操作提示
        if matches!(vo, Vo::Kitty) {
            eprintln!(
                "\nkitty 画面未能启动（当前终端可能不支持）。可试：\n  \
                 claudefm --gui     # 系统窗口，最接近浏览器清晰度\n  \
                 claudefm --tct     # 仍在终端内，但会糊\n"
            );
        }
        return Err(io::Error::other(format!(
            "mpv 退出码 {:?}",
            status.code()
        )));
    }
    Ok(())
}

fn which(bin: &str) -> io::Result<()> {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, bin))
            }
        })
}

fn help() {
    eprintln!(
        "\x1b[38;2;217;119;87mClaude FM\x1b[0m — 终端里播放直播（接近浏览器打开直播页）\n\n\
         用法：\n\
           claudefm           终端全屏（优先 kitty 位图）\n\
           claudefm --gui     系统窗口（清晰度最好）\n\
           claudefm --kitty   强制 kitty\n\
           claudefm --tct     强制字符画面\n\n\
         播放中：q 退出 · 空格 暂停 · 9/0 音量 · f 全屏\n\n\
         不难：就是 mpv 把视频帧画进当前终端。\n\
         要接近浏览器观感，用 Ghostty / WezTerm / Kitty 之一运行。\n"
    );
}
