//! Claude FM — 终端内（或独立窗口）播放 Anthropic Claude FM 直播。

mod player;

use std::env;
use std::io;
use std::process::ExitCode;

use player::{play, print_banner, print_help, VideoMode, CLAUDE_FM_URL};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    match run(&args) {
        Ok(code) => ExitCode::from(code.min(255) as u8),
        Err(err) => {
            eprintln!("claudefm 失败：{err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> io::Result<i32> {
    let mode = VideoMode::resolve(args);
    print_banner(mode);
    play(CLAUDE_FM_URL, mode)
}
