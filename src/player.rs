//! 播放器：终端上半区 kitty 画面，或独立 GUI 窗口；TUI 占用下半区。

use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Claude FM 直播
pub const CLAUDE_FM_URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";

/// 优先 1080p → 720p → 更低（直播 m3u8 合并流）
const YTDL_FORMAT: &str = "96/95/94/best";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    /// 终端上半区 kitty 位图 + 下方 TUI
    KittySplit,
    /// 独立窗口高清 + 全屏 TUI
    Gui,
    /// 仅音频（无画面兜底）
    AudioOnly,
}

impl VideoMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::KittySplit => "终端分屏 · kitty",
            Self::Gui => "独立窗口 · gpu",
            Self::AudioOnly => "仅音频",
        }
    }

    pub fn resolve(args: &[String]) -> Self {
        for a in args {
            match a.as_str() {
                "--gui" | "-g" | "--window" => return Self::Gui,
                "--kitty" | "-k" | "--split" => return Self::KittySplit,
                "--audio" | "-a" => return Self::AudioOnly,
                _ => {}
            }
        }
        if let Ok(v) = std::env::var("CLAUDEFM_VO") {
            match v.to_ascii_lowercase().as_str() {
                "gui" | "gpu" | "window" => return Self::Gui,
                "audio" | "music" => return Self::AudioOnly,
                "kitty" | "split" | "tct" => return Self::KittySplit,
                _ => {}
            }
        }
        // 默认：终端分屏（TUI + 画面同在）
        Self::KittySplit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Idle,
    Starting,
    Playing,
    Stopped,
    Failed,
}

pub struct Player {
    url: String,
    mode: VideoMode,
    child: Option<Child>,
    ipc_path: PathBuf,
    /// kitty 分屏时的画面区域（字符格）
    pub video_rows: u16,
    pub video_cols: u16,
    last_error: Option<String>,
}

impl Player {
    pub fn new(url: impl Into<String>, mode: VideoMode) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let ipc_path = std::env::temp_dir().join(format!("claudefm-{stamp}.sock"));
        Self {
            url: url.into(),
            mode,
            child: None,
            ipc_path,
            video_rows: 0,
            video_cols: 0,
            last_error: None,
        }
    }

    pub fn mode(&self) -> VideoMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: VideoMode) {
        self.mode = mode;
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn ipc_path(&self) -> &std::path::Path {
        &self.ipc_path
    }

    /// 按终端尺寸计算画面区（为下方 TUI 留出 tui_rows 行）
    pub fn layout_for(term_rows: u16, term_cols: u16, tui_rows: u16) -> (u16, u16) {
        let tui = tui_rows.max(7).min(term_rows.saturating_sub(4));
        let video_rows = term_rows.saturating_sub(tui).max(4);
        (video_rows, term_cols.max(20))
    }

    pub fn start(&mut self, video_rows: u16, video_cols: u16) -> io::Result<()> {
        self.stop();
        self.video_rows = video_rows;
        self.video_cols = video_cols;
        self.last_error = None;

        // 清理旧 socket
        let _ = std::fs::remove_file(&self.ipc_path);

        let mut cmd = Command::new("mpv");
        cmd.arg(format!("--ytdl-format={YTDL_FORMAT}"))
            .arg("--title=Claude FM")
            .arg("--keep-open=no")
            .arg(format!(
                "--input-ipc-server={}",
                self.ipc_path.display()
            ))
            .arg("--no-terminal") // 不抢 TUI 的按键；控制走 IPC / 我们自己的 TUI
            .arg("--really-quiet")
            .arg(&self.url);

        match self.mode {
            VideoMode::KittySplit => {
                // 画面画在终端上半区，不进 alt-screen，避免盖住下方 TUI
                cmd.arg("--vo=kitty")
                    .arg("--hwdec=no")
                    .arg("--force-window=no")
                    .arg("--vo-kitty-alt-screen=no")
                    .arg("--vo-kitty-use-shm=yes")
                    .arg("--vo-kitty-top=1")
                    .arg("--vo-kitty-left=1")
                    .arg(format!("--vo-kitty-rows={video_rows}"))
                    .arg(format!("--vo-kitty-cols={video_cols}"))
                    .arg("--keepaspect=yes");

                // kitty 协议必须写到真实 tty，不能和 ratatui 抢同一 stdout 缓冲
                let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
                let tty_err = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
                cmd.stdin(Stdio::null())
                    .stdout(Stdio::from(tty))
                    .stderr(Stdio::from(tty_err));
            }
            VideoMode::Gui => {
                cmd.arg("--vo=gpu-next")
                    .arg("--hwdec=auto")
                    .arg("--force-window=yes")
                    .arg("--geometry=70%")
                    .arg("--keepaspect=yes")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
            }
            VideoMode::AudioOnly => {
                cmd.arg("--no-video")
                    .arg("--force-window=no")
                    .arg("--vo=null")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
            }
        }

        match cmd.spawn() {
            Ok(child) => {
                self.child = Some(child);
                Ok(())
            }
            Err(e) => {
                self.last_error = Some(e.to_string());
                Err(e)
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // 尽量优雅：IPC quit，再 kill
            let _ = self.ipc_cmd("quit");
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(&self.ipc_path);
    }

    pub fn toggle_pause(&self) {
        let _ = self.ipc_cmd("cycle pause");
    }

    pub fn state(&mut self) -> PlayState {
        let Some(child) = self.child.as_mut() else {
            return PlayState::Idle;
        };
        match child.try_wait() {
            Ok(None) => PlayState::Playing,
            Ok(Some(status)) => {
                self.child = None;
                let _ = std::fs::remove_file(&self.ipc_path);
                if status.success() {
                    PlayState::Stopped
                } else {
                    self.last_error = Some(format!("mpv 退出码 {:?}", status.code()));
                    PlayState::Failed
                }
            }
            Err(e) => {
                self.child = None;
                self.last_error = Some(e.to_string());
                PlayState::Failed
            }
        }
    }

    fn ipc_cmd(&self, command: &str) -> io::Result<()> {
        use std::io::Write;
        use std::os::unix::net::UnixStream;
        use std::time::Duration;

        if !self.ipc_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "ipc 未就绪"));
        }
        let mut stream = UnixStream::connect(&self.ipc_path)?;
        let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
        // "cycle pause" → {"command":["cycle","pause"]}
        let parts: Vec<_> = command.split_whitespace().collect();
        let arr = parts
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(",");
        let payload = format!("{{\"command\":[{arr}]}}\n");
        stream.write_all(payload.as_bytes())?;
        Ok(())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn mpv_available() -> bool {
    Command::new("which")
        .arg("mpv")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
