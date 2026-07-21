//! 播放后端：优先 mpv（自带 ytdl），否则 yt-dlp 管道喂给 ffplay。

use std::io;
use std::process::{Child, Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerBackend {
    Mpv,
    Ffplay,
    Missing,
}

impl PlayerBackend {
    pub fn detect() -> Self {
        if command_exists("mpv") {
            Self::Mpv
        } else if command_exists("ffplay") && command_exists("yt-dlp") {
            Self::Ffplay
        } else if command_exists("ffplay") {
            // 没有 yt-dlp 时仍尝试让 ffplay 直接打开 URL（多数情况对 YouTube 无效）
            Self::Ffplay
        } else {
            Self::Missing
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Mpv => "mpv + yt-dlp",
            Self::Ffplay => "yt-dlp | ffplay",
            Self::Missing => "未检测到播放器",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    Idle,
    Starting,
    Playing,
    Exited,
    Failed,
}

pub struct Player {
    url: String,
    backend: PlayerBackend,
    child: Option<Child>,
    /// ffplay 管道模式下还要保留 yt-dlp 子进程
    feeder: Option<Child>,
    last_status: PlayerStatus,
}

impl Player {
    pub fn new(url: String, backend: PlayerBackend) -> Self {
        Self {
            url,
            backend,
            child: None,
            feeder: None,
            last_status: PlayerStatus::Idle,
        }
    }

    pub fn start(&mut self) -> io::Result<()> {
        self.stop();

        match self.backend {
            PlayerBackend::Missing => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "mpv 与 ffplay 均不可用",
                ));
            }
            PlayerBackend::Mpv => {
                // mpv 内置 ytdl：有声有画，独立窗口播放 YouTube 直播
                let child = Command::new("mpv")
                    .args([
                        "--force-window=yes",
                        "--keep-open=no",
                        "--ytdl-format=bestvideo+bestaudio/best",
                        "--title=Claude FM",
                        "--really-quiet",
                        &self.url,
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
                self.child = Some(child);
            }
            PlayerBackend::Ffplay => {
                // yt-dlp 抽流 → ffplay 弹窗播放
                if command_exists("yt-dlp") {
                    let mut ytdlp = Command::new("yt-dlp")
                        .args([
                            "-f",
                            "bestvideo+bestaudio/best",
                            "-o",
                            "-",
                            "--quiet",
                            "--no-warnings",
                            &self.url,
                        ])
                        .stdin(Stdio::null())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()?;

                    let stdout = ytdlp.stdout.take().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "无法连接 yt-dlp 输出管道")
                    })?;

                    let ffplay = Command::new("ffplay")
                        .args([
                            "-autoexit",
                            "-window_title",
                            "Claude FM",
                            "-loglevel",
                            "quiet",
                            "-i",
                            "pipe:0",
                        ])
                        .stdin(stdout)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()?;

                    self.feeder = Some(ytdlp);
                    self.child = Some(ffplay);
                } else {
                    let child = Command::new("ffplay")
                        .args([
                            "-autoexit",
                            "-window_title",
                            "Claude FM",
                            "-loglevel",
                            "quiet",
                            &self.url,
                        ])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()?;
                    self.child = Some(child);
                }
            }
        }

        self.last_status = PlayerStatus::Starting;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(mut feeder) = self.feeder.take() {
            let _ = feeder.kill();
            let _ = feeder.wait();
        }
        self.last_status = PlayerStatus::Idle;
    }

    pub fn status(&mut self) -> PlayerStatus {
        let Some(child) = self.child.as_mut() else {
            self.last_status = PlayerStatus::Idle;
            return PlayerStatus::Idle;
        };

        match child.try_wait() {
            Ok(None) => {
                self.last_status = PlayerStatus::Playing;
                PlayerStatus::Playing
            }
            Ok(Some(code)) => {
                // 清理 feeder
                if let Some(mut feeder) = self.feeder.take() {
                    let _ = feeder.kill();
                    let _ = feeder.wait();
                }
                self.child = None;
                self.last_status = if code.success() {
                    PlayerStatus::Exited
                } else {
                    PlayerStatus::Failed
                };
                self.last_status
            }
            Err(_) => {
                self.child = None;
                if let Some(mut feeder) = self.feeder.take() {
                    let _ = feeder.kill();
                    let _ = feeder.wait();
                }
                self.last_status = PlayerStatus::Failed;
                PlayerStatus::Failed
            }
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
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
