//! 主循环：下方 Claude Code 风 TUI + 上方/侧翼视频。

use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use ratatui::{TerminalOptions, Viewport};

use crate::player::{mpv_available, PlayState, Player, VideoMode, CLAUDE_FM_URL};
use crate::theme::Theme;
use crate::ui::{self, ViewModel};

/// 控制面板占用的行数（其余给画面）
const TUI_ROWS: u16 = 12;

pub struct App {
    player: Player,
    theme: Theme,
    message: String,
    started_at: Option<Instant>,
    tick: u64,
    term_rows: u16,
    term_cols: u16,
    video_rows: u16,
}

impl App {
    pub fn new(mode: VideoMode) -> Self {
        Self {
            player: Player::new(CLAUDE_FM_URL, mode),
            theme: Theme::default(),
            message: "就绪。".into(),
            started_at: None,
            tick: 0,
            term_rows: 24,
            term_cols: 80,
            video_rows: 12,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        if !mpv_available() {
            eprintln!("未找到 mpv。请先：brew install mpv");
            return Err(io::Error::new(io::ErrorKind::NotFound, "mpv missing"));
        }

        // 主屏（不要 alt-screen）：kitty 画面与 TUI 同屏共存
        enable_raw_mode()?;
        let mut out = stdout();
        // 清屏一次，上半留给视频
        execute!(out, Clear(ClearType::All))?;
        out.flush()?;

        let backend = CrosstermBackend::new(stdout());
        let (rows, cols) = size();
        self.term_rows = rows;
        self.term_cols = cols;
        let (video_rows, video_cols) = Player::layout_for(rows, cols, TUI_ROWS);
        self.video_rows = video_rows;

        let tui_y = video_rows;
        let tui_h = rows.saturating_sub(video_rows).max(TUI_ROWS.min(rows));
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, tui_y, cols, tui_h)),
            },
        )?;

        // 自动开播
        self.start_play(video_rows, video_cols);

        let tick_rate = Duration::from_millis(200);
        let mut last_tick = Instant::now();
        let result: io::Result<()> = loop {
            self.tick = self.tick.wrapping_add(1);
            self.sync_play_state();

            // 尺寸变化：重排 TUI 视口并重启分屏播放器
            let (nr, nc) = size();
            if nr != self.term_rows || nc != self.term_cols {
                self.term_rows = nr;
                self.term_cols = nc;
                let (vr, vc) = Player::layout_for(nr, nc, TUI_ROWS);
                self.video_rows = vr;
                let tui_y = vr;
                let tui_h = nr.saturating_sub(vr).max(7);
                // Fixed 视口在 with_options 后不可热改；重建 terminal
                drop(terminal);
                execute!(stdout(), Clear(ClearType::All))?;
                let backend = CrosstermBackend::new(stdout());
                terminal = Terminal::with_options(
                    backend,
                    TerminalOptions {
                        viewport: Viewport::Fixed(Rect::new(0, tui_y, nc, tui_h)),
                    },
                )?;
                if matches!(self.player.mode(), VideoMode::KittySplit)
                    && matches!(self.player.state(), PlayState::Playing | PlayState::Starting)
                {
                    self.start_play(vr, vc);
                }
            }

            let elapsed = self
                .started_at
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);

            let vm = ViewModel {
                play: self.player.state(),
                mode: self.player.mode(),
                message: &self.message,
                elapsed_secs: elapsed,
                tick: self.tick,
                video_rows: self.video_rows,
                term_rows: self.term_rows,
                term_cols: self.term_cols,
            };

            terminal.draw(|f| ui::draw(f, &self.theme, &vm))?;

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
                            self.player.stop();
                            break Ok(());
                        }
                        (KeyCode::Char(' '), _) | (KeyCode::Enter, _) => {
                            match self.player.state() {
                                PlayState::Playing | PlayState::Starting => {
                                    // 空格：mpv 暂停切换；再按一次 stop 用 s
                                    self.player.toggle_pause();
                                    self.message = "已发送暂停/继续。".into();
                                }
                                _ => {
                                    let (vr, vc) = Player::layout_for(
                                        self.term_rows,
                                        self.term_cols,
                                        TUI_ROWS,
                                    );
                                    self.start_play(vr, vc);
                                }
                            }
                        }
                        (KeyCode::Char('s'), _) => {
                            self.player.stop();
                            self.started_at = None;
                            self.message = "已停止。空格重新播放。".into();
                        }
                        (KeyCode::Char('r'), _) => {
                            let (vr, vc) =
                                Player::layout_for(self.term_rows, self.term_cols, TUI_ROWS);
                            self.start_play(vr, vc);
                        }
                        (KeyCode::Char('g'), _) => {
                            self.player.stop();
                            self.player.set_mode(VideoMode::Gui);
                            let (vr, vc) =
                                Player::layout_for(self.term_rows, self.term_cols, TUI_ROWS);
                            // GUI 模式：先清掉终端里的旧画面
                            execute!(stdout(), Clear(ClearType::All))?;
                            self.start_play(vr, vc);
                            self.message = "已切换独立窗口模式。".into();
                        }
                        (KeyCode::Char('k'), _) => {
                            self.player.stop();
                            self.player.set_mode(VideoMode::KittySplit);
                            execute!(stdout(), Clear(ClearType::All))?;
                            let (vr, vc) =
                                Player::layout_for(self.term_rows, self.term_cols, TUI_ROWS);
                            self.video_rows = vr;
                            self.start_play(vr, vc);
                            self.message = "已切换终端分屏（上画面 / 下面板）。".into();
                        }
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        };

        // 还原终端
        self.player.stop();
        disable_raw_mode()?;
        // 不使用 EnterAlternateScreen，故无需 Leave；清屏收尾
        execute!(stdout(), Clear(ClearType::All))?;
        let _ = result;
        Ok(())
    }

    fn start_play(&mut self, video_rows: u16, video_cols: u16) {
        self.video_rows = video_rows;
        self.message = match self.player.mode() {
            VideoMode::KittySplit => "连接直播，画面在上方…".into(),
            VideoMode::Gui => "连接直播，画面在独立窗口…".into(),
            VideoMode::AudioOnly => "连接直播（仅音频）…".into(),
        };
        self.started_at = Some(Instant::now());
        match self.player.start(video_rows, video_cols) {
            Ok(()) => {
                self.message = match self.player.mode() {
                    VideoMode::KittySplit => {
                        "播放中 · 上半画面 · 下半控制。空格暂停，s 停止，q 退出。".into()
                    }
                    VideoMode::Gui => "播放中 · 独立窗口。空格暂停，s 停止，q 退出。".into(),
                    VideoMode::AudioOnly => "播放中（仅音频）。".into(),
                };
            }
            Err(e) => {
                self.started_at = None;
                self.message = format!("启动失败：{e}");
            }
        }
    }

    fn sync_play_state(&mut self) {
        match self.player.state() {
            PlayState::Failed => {
                if let Some(err) = self.player.last_error() {
                    self.message = format!("播放失败：{err} · 按 r 重试，g 改窗口模式");
                }
                self.started_at = None;
            }
            PlayState::Stopped => {
                self.message = "播放结束。空格重播。".into();
                self.started_at = None;
            }
            _ => {}
        }
    }
}

fn size() -> (u16, u16) {
    crossterm::terminal::size()
        .map(|(c, r)| (r, c))
        .unwrap_or((24, 80))
}


