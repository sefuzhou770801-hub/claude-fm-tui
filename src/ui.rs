//! Claude Code 风格控制面板（固定在终端下半区）。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;

use crate::player::{PlayState, VideoMode};
use crate::theme::Theme;

pub struct ViewModel<'a> {
    pub play: PlayState,
    pub mode: VideoMode,
    pub message: &'a str,
    pub elapsed_secs: u64,
    pub tick: u64,
    pub video_rows: u16,
    pub term_rows: u16,
    pub term_cols: u16,
}

pub fn draw(frame: &mut Frame, theme: &Theme, vm: &ViewModel<'_>) {
    let area = frame.area();
    // 整块控制区背景感：靠边框与留白，不刷全屏底以免冲掉上方 kitty 画面
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 顶部分割线标题
            Constraint::Length(3), // 状态
            Constraint::Min(2),    // 详情
            Constraint::Length(2), // 快捷键
        ])
        .split(area);

    draw_header(frame, chunks[0], theme, vm);
    draw_status(frame, chunks[1], theme, vm);
    draw_body(frame, chunks[2], theme, vm);
    draw_keys(frame, chunks[3], theme);
}

fn draw_header(frame: &mut Frame, area: Rect, theme: &Theme, vm: &ViewModel<'_>) {
    let mode_tag = match vm.mode {
        VideoMode::KittySplit => "split",
        VideoMode::Gui => "gui",
        VideoMode::AudioOnly => "audio",
    };
    let line = Line::from(vec![
        Span::styled("● ", theme.title()),
        Span::styled("Claude FM", theme.title()),
        Span::styled("  ", theme.dim()),
        Span::styled(
            format!("/{mode_tag}"),
            Style::default().fg(theme.muted).add_modifier(Modifier::DIM),
        ),
        Span::styled(
            format!("  ·  video {}×{}", vm.video_rows, vm.term_cols),
            theme.dim(),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_status(frame: &mut Frame, area: Rect, theme: &Theme, vm: &ViewModel<'_>) {
    let (label, style) = match vm.play {
        PlayState::Playing | PlayState::Starting => {
            let spin = spinner(vm.tick);
            (format!("{spin}  playing"), theme.success())
        }
        PlayState::Idle | PlayState::Stopped => ("○  idle".into(), theme.dim()),
        PlayState::Failed => ("×  failed".into(), theme.error()),
    };

    let elapsed = format_duration(vm.elapsed_secs);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(Span::styled(" status ", theme.dim()));

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(block.inner(area));

    frame.render_widget(block, area);

    let left = Paragraph::new(Line::from(vec![
        Span::styled(format!("  {label}"), style),
        Span::styled(format!("   {elapsed}"), theme.dim()),
    ]));
    frame.render_widget(left, inner[0]);

    // 右侧伪波形 — Claude Code 式克制动效
    let ratio = match vm.play {
        PlayState::Playing => {
            let t = (vm.tick % 24) as f64 / 24.0;
            0.25 + 0.55 * (t * std::f64::consts::PI * 2.0).sin().abs()
        }
        PlayState::Starting => ((vm.tick % 12) as f64) / 12.0 * 0.6,
        _ => 0.05,
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(theme.accent).bg(theme.border))
        .ratio(ratio.clamp(0.0, 1.0))
        .label("");
    // 给 gauge 一点边距
    let g = Rect {
        x: inner[1].x + 1,
        y: inner[1].y,
        width: inner[1].width.saturating_sub(2),
        height: 1,
    };
    if g.width > 0 {
        frame.render_widget(gauge, g);
    }
}

fn draw_body(frame: &mut Frame, area: Rect, theme: &Theme, vm: &ViewModel<'_>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border())
        .title(Span::styled(" session ", theme.dim()));

    let lines = vec![
        Line::from(vec![
            Span::styled("  title  ", theme.dim()),
            Span::styled(
                "music for thinking and building",
                theme.text().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  source ", theme.dim()),
            Span::styled(
                "youtube.com/live/tRsQsTMvPNg",
                Style::default().fg(theme.accent_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("  layout ", theme.dim()),
            Span::styled(
                format!(
                    "上半 {} 行画面 · 下半 {} 行面板 · 共 {}×{}",
                    vm.video_rows,
                    vm.term_rows.saturating_sub(vm.video_rows),
                    vm.term_cols,
                    vm.term_rows
                ),
                theme.dim(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  note   ", theme.dim()),
            Span::styled(vm.message, theme.text()),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn draw_keys(frame: &mut Frame, area: Rect, theme: &Theme) {
    let items = [
        ("space", "播放/暂停"),
        ("r", "重连"),
        ("g", "切窗口"),
        ("k", "切分屏"),
        ("q", "退出"),
    ];
    let mut spans = Vec::new();
    for (i, (k, label)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", theme.dim()));
        }
        spans.push(Span::styled(format!("[{k}]"), theme.key()));
        spans.push(Span::styled(format!(" {label}"), theme.dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
