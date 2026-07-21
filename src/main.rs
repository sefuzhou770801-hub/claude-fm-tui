//! Claude FM — 在当前终端窗口内播放 Anthropic Claude FM 直播（有声有画）。
//!
//! 画面由 mpv 的终端渲染输出（kitty 协议或 tct 真彩色字符）直接画在本终端。

mod player;

use std::io;
use std::process::ExitCode;

use player::{play_in_terminal, print_banner, TerminalVo, CLAUDE_FM_URL};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("claudefm 失败：{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<i32> {
    let vo = TerminalVo::detect();
    print_banner(vo);

    // 前台占用当前终端：视频帧画在这里，q 退出 mpv 后本程序结束
    let code = play_in_terminal(CLAUDE_FM_URL, vo)?;
    Ok(code)
}
