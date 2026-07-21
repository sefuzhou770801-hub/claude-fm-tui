//! Claude FM — Claude Code 风格 TUI + 终端内（或窗口）视频同屏。

mod app;
mod player;
mod theme;
mod ui;

use std::env;
use std::process::ExitCode;

use app::App;
use player::VideoMode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    let mode = VideoMode::resolve(&args);
    let mut app = App::new(mode);
    match app.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("claudefm 失败：{err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    eprintln!(
        "\x1b[38;2;217;119;87m● Claude FM\x1b[0m  —  Claude Code 风格面板 + 直播画面\n\n\
         用法：\n\
           claudefm           默认：上半终端画面(kitty) + 下半 TUI\n\
           claudefm --kitty   同上（显式分屏）\n\
           claudefm --gui     独立高清窗口 + 全宽 TUI\n\
           claudefm --audio   仅音频 + TUI\n\n\
         面板快捷键：\n\
           space  播放/暂停    s  停止    r  重连\n\
           g      切到窗口     k  切回分屏    q  退出\n\n\
         环境变量：CLAUDEFM_VO=kitty|gui|audio\n\n\
         说明：分屏依赖终端的 kitty 图形协议（Ghostty / WezTerm / Kitty 等）。\n\
         若不支持，按 g 或使用 --gui。\n"
    );
}
