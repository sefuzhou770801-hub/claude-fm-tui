//! Claude FM — 终端分屏播放（形态 D 定稿）
//!
//! 上：直播画面铺满（kitty 位图）
//! 下：仅 1 行状态栏
//!
//! 真机依赖支持 kitty 图形协议的终端（Ghostty / WezTerm / Kitty）。
//! 失败时可 `claudefm --gui` / `claudefm --tct`。

use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const URL: &str = "https://www.youtube.com/live/tRsQsTMvPNg";
/// 直播：优先 1080p → 720p → 更低
const FORMAT: &str = "96/95/94/best";
/// 底部状态栏固定 1 行
const STATUS_ROWS: u16 = 1;

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

    let mode = Mode::resolve(&args);
    match run(mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("claudefm 失败：{e}");
            if matches!(mode, Mode::Split) {
                eprintln!(
                    "分屏需要 kitty 图形协议终端（Ghostty/WezTerm/Kitty）。\n\
                     可试：claudefm --gui   或  claudefm --tct"
                );
            }
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// 定稿：上画面 + 底 1 行状态
    Split,
    /// 兜底：系统窗口
    Gui,
    /// 兜底：全屏色块
    Tct,
}

impl Mode {
    fn resolve(args: &[String]) -> Self {
        for a in args {
            match a.as_str() {
                "--gui" | "-g" => return Self::Gui,
                "--tct" => return Self::Tct,
                "--split" | "-d" | "--kitty" => return Self::Split,
                _ => {}
            }
        }
        if let Ok(v) = env::var("CLAUDEFM_VO") {
            match v.to_ascii_lowercase().as_str() {
                "gui" | "gpu" | "window" => return Self::Gui,
                "tct" | "ansi" => return Self::Tct,
                _ => {}
            }
        }
        // 默认：用户选定的 D
        Self::Split
    }
}

fn run(mode: Mode) -> io::Result<()> {
    match mode {
        Mode::Split => run_split(),
        Mode::Gui => run_fullscreen_mpv(Mode::Gui),
        Mode::Tct => run_fullscreen_mpv(Mode::Tct),
    }
}

/// 分屏：mpv 画上面，主进程刷底部 1 行状态；q 退出。
fn run_split() -> io::Result<()> {
    let (cols, rows) = term_size();
    if rows < 4 {
        return Err(io::Error::other("终端太矮，至少需要 4 行"));
    }
    let video_rows = rows.saturating_sub(STATUS_ROWS);

    // 清屏；光标藏起
    let mut out = io::stdout();
    write!(out, "\x1b[2J\x1b[H\x1b[?25l")?;
    out.flush()?;

    let ipc = ipc_path();
    let _ = std::fs::remove_file(&ipc);

    let mut child = spawn_split_mpv(video_rows, cols, &ipc)?;
    let started = Instant::now();
    let running = Arc::new(AtomicBool::new(true));

    // 状态栏刷新线程（只写最后一行，不动上方画面区）
    let flag = Arc::clone(&running);
    let status_handle = thread::spawn(move || {
        let mut tick = 0u64;
        while flag.load(Ordering::Relaxed) {
            let elapsed = started.elapsed().as_secs();
            let spin = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".chars().nth(tick as usize % 10).unwrap_or('●');
            // 移到最后一行、整行清除、写状态
            let line = format!(
                "\x1b[{row};1H\x1b[2K\x1b[38;2;217;119;87m{spin} Claude FM\x1b[0m  \
                 \x1b[32m▶ live\x1b[0m  {time}  \
                 \x1b[2m{vr}×{vc}\x1b[0m  \
                 \x1b[2m[q]\x1b[0m退出 \x1b[2m[空格]\x1b[0m暂停 \x1b[2m[9/0]\x1b[0m音量",
                row = rows,
                time = fmt_time(elapsed),
                vr = video_rows,
                vc = cols,
            );
            let mut o = io::stdout();
            let _ = write!(o, "{line}");
            let _ = o.flush();
            tick += 1;
            thread::sleep(Duration::from_millis(200));
        }
    });

    // 按键：转给 mpv IPC；q 结束
    let result = key_loop(&ipc, &mut child);

    running.store(false, Ordering::Relaxed);
    let _ = status_handle.join();
    stop_child(&mut child, &ipc);

    // 还原终端
    let mut out = io::stdout();
    write!(out, "\x1b[?25h\x1b[2J\x1b[H")?;
    out.flush()?;

    result
}

fn spawn_split_mpv(video_rows: u16, cols: u16, ipc: &std::path::Path) -> io::Result<Child> {
    let tty_out = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let tty_err = OpenOptions::new().read(true).write(true).open("/dev/tty")?;

    Command::new("mpv")
        .args([
            "--vo=kitty",
            "--vo-kitty-alt-screen=no",
            "--vo-kitty-use-shm=yes",
            "--vo-kitty-top=1",
            "--vo-kitty-left=1",
            &format!("--vo-kitty-rows={video_rows}"),
            &format!("--vo-kitty-cols={cols}"),
            "--hwdec=no",
            "--force-window=no",
            "--keepaspect=yes",
            "--no-terminal", // 按键由我们转发，避免抢状态行
            &format!("--input-ipc-server={}", ipc.display()),
            &format!("--ytdl-format={FORMAT}"),
            "--title=Claude FM",
            "--really-quiet",
            "--msg-level=all=no",
            URL,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(tty_out))
        .stderr(Stdio::from(tty_err))
        .spawn()
}

fn key_loop(ipc: &std::path::Path, child: &mut Child) -> io::Result<()> {
    use std::io::Read;
    // raw mode
    let raw = RawMode::enter()?;
    let mut stdin = io::stdin();
    let mut buf = [0u8; 16];

    loop {
        // mpv 已退出？
        match child.try_wait()? {
            Some(status) => {
                drop(raw);
                if status.success() {
                    return Ok(());
                }
                return Err(io::Error::other(format!("mpv 退出 {:?}", status.code())));
            }
            None => {}
        }

        // 非阻塞读有点麻烦；短超时用 poll 思路：termios + 小 sleep
        // 这里用阻塞 read 单字节（raw）
        // 为了能轮询子进程，设非阻塞 stdin
        if wait_stdin(Duration::from_millis(150))? {
            let n = stdin.read(&mut buf)?;
            if n == 0 {
                continue;
            }
            let ch = buf[0] as char;
            match ch {
                'q' | 'Q' | '\x03' => {
                    drop(raw);
                    return Ok(());
                }
                ' ' => {
                    let _ = ipc_cmd(ipc, &["cycle", "pause"]);
                }
                '9' => {
                    let _ = ipc_cmd(ipc, &["add", "volume", "-5"]);
                }
                '0' => {
                    let _ = ipc_cmd(ipc, &["add", "volume", "5"]);
                }
                'm' | 'M' => {
                    let _ = ipc_cmd(ipc, &["cycle", "mute"]);
                }
                'f' | 'F' => {
                    // 分屏模式无系统全屏意义不大，忽略
                }
                _ => {}
            }
        }
    }
}

fn run_fullscreen_mpv(mode: Mode) -> io::Result<()> {
    let mut cmd = Command::new("mpv");
    cmd.arg(format!("--ytdl-format={FORMAT}"))
        .arg("--title=Claude FM")
        .arg(URL);

    match mode {
        Mode::Gui => {
            cmd.args([
                "--vo=gpu-next",
                "--hwdec=auto",
                "--force-window=yes",
                "--geometry=80%",
            ]);
        }
        Mode::Tct => {
            cmd.args([
                "--vo=tct",
                "--vo-tct-algo=half-blocks",
                "--vo-tct-256=no",
                "--hwdec=no",
                "--force-window=no",
            ]);
        }
        Mode::Split => unreachable!(),
    }

    let st = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if st.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("mpv 退出 {:?}", st.code())))
    }
}

fn stop_child(child: &mut Child, ipc: &std::path::Path) {
    let _ = ipc_cmd(ipc, &["quit"]);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(ipc);
}

fn ipc_cmd(ipc: &std::path::Path, parts: &[&str]) -> io::Result<()> {
    use std::os::unix::net::UnixStream;
    if !ipc.exists() {
        return Ok(());
    }
    let mut s = UnixStream::connect(ipc)?;
    s.set_write_timeout(Some(Duration::from_millis(150)))?;
    let arr = parts
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(",");
    let payload = format!("{{\"command\":[{arr}]}}\n");
    s.write_all(payload.as_bytes())?;
    Ok(())
}

fn ipc_path() -> PathBuf {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    env::temp_dir().join(format!("claudefm-{t}.sock"))
}

fn term_size() -> (u16, u16) {
    // (cols, rows)
    if let Ok((c, r)) = crossterm_size_fallback() {
        return (c, r);
    }
    (80, 24)
}

fn crossterm_size_fallback() -> io::Result<(u16, u16)> {
    // 不引入 crossterm 依赖：用 stty / TIOCGWINSZ
    unsafe {
        let mut ws: libc_winsize = std::mem::zeroed();
        if libc_ioctl(1, libc_tiocgwinsz(), &mut ws as *mut _ as *mut _) == 0 && ws.ws_col > 0 {
            return Ok((ws.ws_col, ws.ws_row));
        }
    }
    // stty
    let out = Command::new("stty").args(["size"]).output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut it = s.split_whitespace();
    let rows: u16 = it.next().and_then(|x| x.parse().ok()).unwrap_or(24);
    let cols: u16 = it.next().and_then(|x| x.parse().ok()).unwrap_or(80);
    Ok((cols, rows))
}

// 最小 ioctl 绑定，避免依赖 libc crate
#[repr(C)]
struct libc_winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(unix)]
fn libc_tiocgwinsz() -> u64 {
    // macOS TIOCGWINSZ
    #[cfg(target_os = "macos")]
    {
        0x40087468
    }
    #[cfg(not(target_os = "macos"))]
    {
        0x5413
    }
}

#[cfg(unix)]
unsafe fn libc_ioctl(fd: i32, req: u64, arg: *mut libc_winsize) -> i32 {
    extern "C" {
        fn ioctl(fd: i32, req: u64, ...) -> i32;
    }
    ioctl(fd, req, arg)
}

fn fmt_time(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
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
        "\x1b[38;2;217;119;87m● Claude FM\x1b[0m — 上半直播画面 · 底栏一行状态\n\n\
         用法：\n\
           claudefm           默认分屏（形态 D）\n\
           claudefm --gui     系统窗口高清\n\
           claudefm --tct     全屏像素色块\n\n\
         分屏快捷键：\n\
           q 退出  空格 暂停  9/0 音量  m 静音\n\n\
         需要 Ghostty / WezTerm / Kitty。\n"
    );
}

// ── raw mode ─────────────────────────────────────────

struct RawMode {
    fd: i32,
    original: termios_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct termios_t {
    c_iflag: u64,
    c_oflag: u64,
    c_cflag: u64,
    c_lflag: u64,
    c_cc: [u8; 20],
    c_ispeed: u64,
    c_ospeed: u64,
}

// macOS termios 布局因架构而异；用 stty 更稳
// 改用标准方式：libc via Command `stty raw` / `stty cooked` 太粗
// 使用 crossterm 会更干净——加回最小依赖

impl RawMode {
    fn enter() -> io::Result<Self> {
        // 用 stty 进入 raw（prototype 级够用）
        let _ = Command::new("stty")
            .args(["-echo", "raw", "min", "0", "time", "1"])
            .status();
        Ok(Self {
            fd: 0,
            original: unsafe { std::mem::zeroed() },
        })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = Command::new("stty").args(["sane"]).status();
        let _ = self.fd;
        let _ = self.original;
    }
}

fn wait_stdin(timeout: Duration) -> io::Result<bool> {
    // poll stdin with select-like sleep + nonblocking attempt
    // stty time=1 already makes read return quickly; just sleep a bit if needed
    thread::sleep(timeout.min(Duration::from_millis(50)));
    // Always try read in key_loop after this; use a better approach:

    // Actually use libc poll on fd 0
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    const POLLIN: i16 = 0x1;
    extern "C" {
        fn poll(fds: *mut PollFd, nfds: u32, timeout: i32) -> i32;
    }
    let mut pfd = PollFd {
        fd: 0,
        events: POLLIN,
        revents: 0,
    };
    let ms = timeout.as_millis() as i32;
    let r = unsafe { poll(&mut pfd, 1, ms) };
    Ok(r > 0 && (pfd.revents & POLLIN) != 0)
}
