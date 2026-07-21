//! Claude FM — 终端里一键收听 Anthropic 的 Claude FM 直播。
//!
//! 播放链路：`yt-dlp` 解析 YouTube 直播 → `mpv`（优先）或 `ffplay` 出声。

mod player;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use player::{Player, PlayerBackend, PlayerStatus};
use ui::{draw, AppView, StatusKind};

/// Claude FM 官方直播固定地址（任务要求只支持这一个源）
const CLAUDE_FM_URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";
const CLAUDE_FM_TITLE: &str = "Claude FM 🎵 music for thinking and building";

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let backend = PlayerBackend::detect();
    let mut player = Player::new(CLAUDE_FM_URL.to_string(), backend.clone());

    let mut view = AppView {
        title: CLAUDE_FM_TITLE.to_string(),
        url: CLAUDE_FM_URL.to_string(),
        backend_label: backend.label().to_string(),
        status: StatusKind::Idle,
        message: match backend {
            PlayerBackend::Missing => {
                "未找到 mpv 或 ffplay。请安装：brew install mpv  或确认 ffmpeg 在 PATH 中。"
                    .into()
            }
            _ => "按 Enter / 空格 开始播放，q 退出。".into(),
        },
        started_at: None,
        tick: 0,
    };

    // 有可用后端时自动开始播放
    if !matches!(backend, PlayerBackend::Missing) {
        start_playback(&mut player, &mut view);
    }

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        view.tick = view.tick.wrapping_add(1);
        refresh_status(&mut player, &mut view);
        terminal.draw(|frame| draw(frame, &view))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        player.stop();
                        break;
                    }
                    (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
                        match player.status() {
                            PlayerStatus::Playing | PlayerStatus::Starting => {
                                player.stop();
                                view.status = StatusKind::Idle;
                                view.message = "已停止。按 Enter / 空格 重新播放。".into();
                                view.started_at = None;
                            }
                            PlayerStatus::Idle | PlayerStatus::Exited | PlayerStatus::Failed => {
                                if matches!(backend, PlayerBackend::Missing) {
                                    view.status = StatusKind::Error;
                                    view.message = "没有可用播放器（需要 mpv 或 ffplay）。".into();
                                } else {
                                    start_playback(&mut player, &mut view);
                                }
                            }
                        }
                    }
                    (KeyCode::Char('r'), _) => {
                        player.stop();
                        if matches!(backend, PlayerBackend::Missing) {
                            view.status = StatusKind::Error;
                            view.message = "没有可用播放器（需要 mpv 或 ffplay）。".into();
                        } else {
                            start_playback(&mut player, &mut view);
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn start_playback(player: &mut Player, view: &mut AppView) {
    view.status = StatusKind::Connecting;
    view.message = "正在连接 Claude FM 直播…".into();
    view.started_at = Some(Instant::now());
    match player.start() {
        Ok(()) => {
            view.status = StatusKind::Playing;
            view.message = "播放中。按 Enter / 空格 停止，r 重连，q 退出。".into();
        }
        Err(err) => {
            view.status = StatusKind::Error;
            view.message = format!("启动失败：{err}");
            view.started_at = None;
        }
    }
}

fn refresh_status(player: &mut Player, view: &mut AppView) {
    match player.status() {
        PlayerStatus::Playing | PlayerStatus::Starting => {
            if !matches!(view.status, StatusKind::Playing | StatusKind::Connecting) {
                view.status = StatusKind::Playing;
            }
        }
        PlayerStatus::Exited => {
            if matches!(view.status, StatusKind::Playing | StatusKind::Connecting) {
                view.status = StatusKind::Idle;
                view.message = "播放器已退出。按 Enter / 空格 重新播放。".into();
                view.started_at = None;
            }
        }
        PlayerStatus::Failed => {
            if !matches!(view.status, StatusKind::Error) {
                view.status = StatusKind::Error;
                view.message = "播放器异常退出。按 r 重试。".into();
                view.started_at = None;
            }
        }
        PlayerStatus::Idle => {}
    }
}
