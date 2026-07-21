//! Claude FM 终端界面。

use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Idle,
    Connecting,
    Playing,
    Error,
}

pub struct AppView {
    pub title: String,
    pub url: String,
    pub backend_label: String,
    pub status: StatusKind,
    pub message: String,
    pub started_at: Option<Instant>,
    pub tick: u64,
}

pub fn draw(frame: &mut Frame, view: &AppView) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], view);
    draw_body(frame, chunks[1], view);
    draw_status(frame, chunks[2], view);
    draw_footer(frame, chunks[3]);
}

fn draw_header(frame: &mut Frame, area: Rect, view: &AppView) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Claude FM ",
            Style::default()
                .fg(Color::Rgb(203, 166, 247))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "· terminal radio",
            Style::default().fg(Color::Rgb(108, 112, 134)),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 250)))
            .title(Span::styled(
                " claudefm ",
                Style::default().fg(Color::Rgb(137, 180, 250)),
            )),
    )
    .alignment(Alignment::Left);

    // 右侧显示后端
    let _ = view;
    frame.render_widget(title, area);
}

fn draw_body(frame: &mut Frame, area: Rect, view: &AppView) {
    let pulse = match view.status {
        StatusKind::Playing => spinner(view.tick),
        StatusKind::Connecting => spinner(view.tick),
        StatusKind::Idle => "○",
        StatusKind::Error => "!",
    };

    let (status_label, status_color) = match view.status {
        StatusKind::Idle => ("已停止", Color::Rgb(166, 173, 200)),
        StatusKind::Connecting => ("连接中", Color::Rgb(250, 179, 135)),
        StatusKind::Playing => ("播放中", Color::Rgb(166, 227, 161)),
        StatusKind::Error => ("出错", Color::Rgb(243, 139, 168)),
    };

    let elapsed = view
        .started_at
        .map(|t| format_duration(t.elapsed().as_secs()))
        .unwrap_or_else(|| "--:--".into());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                pulse,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                &view.title,
                Style::default()
                    .fg(Color::Rgb(205, 214, 244))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  状态  "),
            Span::styled(
                status_label,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   已播 {elapsed}"),
                Style::default().fg(Color::Rgb(108, 112, 134)),
            ),
        ]),
        Line::from(vec![
            Span::raw("  后端  "),
            Span::styled(
                &view.backend_label,
                Style::default().fg(Color::Rgb(137, 180, 250)),
            ),
        ]),
        Line::from(vec![
            Span::raw("  源    "),
            Span::styled(&view.url, Style::default().fg(Color::Rgb(148, 226, 213))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(&view.message, Style::default().fg(Color::Rgb(166, 173, 200))),
        ]),
    ];

    let body = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(88, 91, 112)))
                .title(Span::styled(
                    " live ",
                    Style::default().fg(Color::Rgb(166, 173, 200)),
                )),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(body, area);

    // 底部一条「声波」装饰条
    if area.height > 4 {
        let gauge_area = Rect {
            x: area.x + 2,
            y: area.y + area.height.saturating_sub(2),
            width: area.width.saturating_sub(4),
            height: 1,
        };
        let ratio = match view.status {
            StatusKind::Playing => {
                // 伪波形：随 tick 轻微起伏
                let t = (view.tick % 20) as f64 / 20.0;
                0.35 + 0.45 * (t * std::f64::consts::PI * 2.0).sin().abs()
            }
            StatusKind::Connecting => ((view.tick % 10) as f64) / 10.0,
            _ => 0.0,
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Rgb(203, 166, 247)).bg(Color::Rgb(49, 50, 68)))
            .ratio(ratio.clamp(0.0, 1.0))
            .label("");
        frame.render_widget(gauge, gauge_area);
    }
}

fn draw_status(frame: &mut Frame, area: Rect, view: &AppView) {
    let hint = match view.status {
        StatusKind::Playing => "正在直播 · 适合 coding / 写作时的背景声",
        StatusKind::Connecting => "解析 YouTube 直播流，首次可能需要几秒…",
        StatusKind::Idle => "就绪 · 一键进入 Claude FM",
        StatusKind::Error => "检查网络，或确认已安装 mpv / yt-dlp / ffmpeg",
    };

    let p = Paragraph::new(Line::from(Span::styled(
        format!("  {hint}"),
        Style::default().fg(Color::Rgb(166, 173, 200)),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(88, 91, 112))),
    );

    frame.render_widget(p, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let keys = Paragraph::new(Line::from(vec![
        key_span("Enter/Space", "播放/停止"),
        Span::raw("   "),
        key_span("r", "重连"),
        Span::raw("   "),
        key_span("q", "退出"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(88, 91, 112)))
            .title(Span::styled(
                " keys ",
                Style::default().fg(Color::Rgb(108, 112, 134)),
            )),
    );

    frame.render_widget(keys, area);
}

fn key_span(k: &str, label: &str) -> Span<'static> {
    Span::styled(
        format!("[{k}] {label}"),
        Style::default().fg(Color::Rgb(250, 179, 135)),
    )
}

fn spinner(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(tick as usize) % FRAMES.len()]
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
