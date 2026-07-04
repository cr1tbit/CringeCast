use cringecast::core::{
    sanitize, smart_split, AppConfig, AudioBackend, CringeError, CringeService,
    FallbackLanguageDetector, TeapotRequest,
};
use cringecast::platform::linux_shell::LinuxShellBackend;
use rouille::{Request, Response};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::mem;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_secs(5);
const ENCODER_VOLUME_STEP: i16 = 2;

type SharedService = Arc<Mutex<CringeService<LinuxShellBackend, FallbackLanguageDetector>>>;

#[derive(Clone)]
struct AppState {
    root: String,
    service: SharedService,
    audio_tx: SyncSender<AudioCommand>,
    display_tx: SyncSender<LcdMessage>,
    volume_tx: Sender<u8>,
    queue_policy: QueuePolicy,
}

#[derive(Clone, Copy)]
enum QueuePolicy {
    DropNew,
    Block,
}

impl QueuePolicy {
    fn from_env() -> Self {
        match std::env::var("CRINGECAST_QUEUE_POLICY")
            .unwrap_or_else(|_| "drop_new".to_string())
            .trim()
        {
            "block" => Self::Block,
            _ => Self::DropNew,
        }
    }
}

enum AudioCommand {
    Speak {
        saying: String,
        lang_override: Option<String>,
    },
    PlayFile {
        category: String,
        filename_no_ext: String,
    },
}

enum LcdMessage {
    Row0(String),
    Row1(String),
}

fn main() {
    let root = std::env::var("CRINGECAST_ROOT").unwrap_or_else(|_| ".".to_string());
    let port = std::env::var("CRINGECAST_PORT").unwrap_or_else(|_| "42069".to_string());
    let queue_max = std::env::var("CRINGECAST_QUEUE_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.max(1))
        .unwrap_or(2);
    let queue_policy = QueuePolicy::from_env();
    let super_secret_key =
        std::env::var("CRINGECAST_SUPER_SECRET_KEY").unwrap_or_else(|_| "change-me".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let backend = LinuxShellBackend::new(
        format!("{}/CringeCast/shellscripts", root),
        format!("{}/audio_files", root),
        is_arm(),
    );
    let volume_state_path = std::env::var("CRINGECAST_VOLUME_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(format!("{}/volume.txt", root)));
    if let Some(volume) = load_persisted_volume(&volume_state_path) {
        match backend.set_volume(volume) {
            Ok(()) => println!("restored persisted volume: {}%", volume),
            Err(err) => eprintln!("failed to restore persisted volume {}%: {}", volume, err),
        }
    }

    let mut config = AppConfig::default();
    config.super_secret_key = super_secret_key;
    let service = CringeService::new(config.clone(), backend.clone(), FallbackLanguageDetector);
    let service: SharedService = Arc::new(Mutex::new(service));
    let (audio_tx, audio_rx) = mpsc::sync_channel(queue_max);
    spawn_audio_worker(backend, config, audio_rx);
    let lcd_serial_path =
        std::env::var("CRINGECAST_LCD_SERIAL").unwrap_or_else(|_| "/dev/ttyUSB0".to_string());
    let (display_tx, display_rx) = mpsc::sync_channel(8);
    let (volume_tx, volume_rx) = mpsc::channel();
    spawn_lcd_display_worker(
        lcd_serial_path,
        display_rx,
        service.clone(),
        volume_tx.clone(),
        ENCODER_VOLUME_STEP,
    );
    spawn_volume_persist_worker(volume_state_path, volume_rx);

    let state = AppState {
        root: root,
        service,
        audio_tx,
        display_tx,
        volume_tx,
        queue_policy,
    };

    println!("cringecast-server listening on {}", addr);
    println!(
        "audio queue configured: max={}, policy={}",
        queue_max,
        queue_policy_name(state.queue_policy)
    );
    rouille::start_server(addr, move |request| handle_request(&state, request));
}

fn handle_request(state: &AppState, request: &Request) -> Response {
    let url = request.url();
    let raw_path = url.split('?').next().unwrap_or("");

    if raw_path == "/old" || raw_path == "/old/" {
        return serve_file(
            &format!("{}/CringeCast/static/index.html", state.root),
            "text/html; charset=utf-8",
        );
    }

    if raw_path == "/" || raw_path == "/metro" || raw_path == "/metro/" {
        return serve_file(
            &format!("{}/CringeCast/static/metro/index.html", state.root),
            "text/html; charset=utf-8",
        );
    }

    let path_owned = normalize_old_path(raw_path);
    let path_only = path_owned.as_str();

    if path_only == "/favicon.png" {
        return serve_file(
            &format!("{}/CringeCast/static/favicon.png", state.root),
            "image/png",
        );
    }

    if path_only.starts_with("/static/") {
        let relative = path_only.trim_start_matches("/static/");
        let mime = mime_from_path(relative);
        return serve_file(
            &format!("{}/CringeCast/static/{}", state.root, relative),
            mime,
        );
    }

    if path_only == "/teapot/status" {
        return with_service(
            state,
            |svc| {
                let now = SystemTime::now();
                let enabled = svc.teapot_status(now);
                let remaining_seconds = if enabled {
                    svc.teapot.get_remaining(now).as_secs()
                } else {
                    0
                };
                Ok((enabled, remaining_seconds))
            },
            |(enabled, remaining_seconds)| {
                let body = serde_json::json!({
                    "enabled": enabled,
                    "remaining_seconds": remaining_seconds
                })
                .to_string();
                Response::text(body)
            },
        );
    }

    if let Err(resp) = guard_request(state, request) {
        return resp;
    }

    if request.method() == "POST" && path_only == "/uploader" {
        notify_lcd_api_request(state, path_only);
        return handle_uploader(state, request);
    }

    let segments: Vec<&str> = path_only.trim_start_matches('/').split('/').collect();
    notify_lcd_api_request(state, path_only);

    if path_only == "/stop" {
        return with_service(state, |svc| svc.stop(), |()| Response::text("OK: Stopped"));
    }

    if path_only == "/vol" {
        return with_service(
            state,
            |svc| svc.get_volume(),
            |vol| Response::text(vol.to_string()),
        );
    }

    if segments.len() == 2 && segments[0] == "vol" {
        let vol = segments[1].parse::<u8>();
        return match vol {
            Ok(v) => with_service(
                state,
                |svc| svc.set_volume(v),
                |()| {
                    if let Err(err) = state.volume_tx.send(v) {
                        eprintln!("volume persist worker unavailable: {}", err);
                    }
                    notify_lcd_volume(state, v);
                    Response::text(format!("volume set ok: {}%", v))
                },
            ),
            Err(_) => Response::text(format!("invalid volume requested: {}", segments[1]))
                .with_status_code(400),
        };
    }

    if path_only == "/getFilelist" {
        return with_service(
            state,
            |svc| svc.list_files(),
            |files| match serde_json::to_string(&files) {
                Ok(body) => Response::text(body),
                Err(e) => map_error(CringeError::Backend(format!("json encode failed: {}", e))),
            },
        );
    }

    if segments.len() == 2 && segments[0] == "teapot" {
        return with_service_mut(
            state,
            |svc| {
                let access = svc.resolve_access(request.get_param("super_secret_key").as_deref());
                let mode = match segments[1] {
                    "on" => TeapotRequest::On,
                    "off" => TeapotRequest::Off,
                    "permanent" => TeapotRequest::Permanent,
                    _ => return Err(CringeError::InvalidInput("Invalid request".to_string())),
                };
                svc.teapot_control(mode, access, SystemTime::now())
            },
            |msg| Response::text(msg),
        );
    }

    if segments.len() == 2 && segments[0] == "say" {
        return enqueue_audio(
            state,
            AudioCommand::Speak {
                saying: segments[1].to_string(),
                lang_override: Some("en".to_string()),
            },
            format!("OK: {}", segments[1]),
        );
    }

    if segments.len() == 2 && segments[0] == "mow" {
        return enqueue_audio(
            state,
            AudioCommand::Speak {
                saying: segments[1].to_string(),
                lang_override: Some("pl".to_string()),
            },
            format!("OK: {}", segments[1]),
        );
    }

    if segments.len() == 2 && segments[0] == "guess" {
        let lang = request.get_param("l");
        return enqueue_audio(
            state,
            AudioCommand::Speak {
                saying: segments[1].to_string(),
                lang_override: lang.clone(),
            },
            format!("OK: {}, language is: {:?}", segments[1], lang),
        );
    }

    if segments.len() == 3 && segments[0] == "play" {
        return enqueue_audio(
            state,
            AudioCommand::PlayFile {
                category: segments[1].to_string(),
                filename_no_ext: segments[2].to_string(),
            },
            format!("OK: {}/{}.mp3", segments[1], segments[2]),
        );
    }

    if segments.len() == 1 && !segments[0].is_empty() {
        let lang = request.get_param("l");
        return enqueue_audio(
            state,
            AudioCommand::Speak {
                saying: segments[0].to_string(),
                lang_override: lang.clone(),
            },
            format!("OK: {}, language is: {:?}", segments[0], lang),
        );
    }

    Response::empty_404()
}

fn guard_request(state: &AppState, request: &Request) -> Result<(), Response> {
    let url = request.url();
    let raw_path = url.split('?').next().unwrap_or("");
    let path_owned = normalize_old_path(raw_path);
    let path_only = path_owned.as_str();

    if raw_path == "/old" || raw_path == "/old/" {
        return Ok(());
    }

    if path_only == "/"
        || path_only == "/metro"
        || path_only == "/metro/"
        || path_only == "/favicon.png"
        || path_only.starts_with("/static/")
    {
        return Ok(());
    }

    with_service_mut(
        state,
        |svc| {
            let access = svc.resolve_access(request.get_param("super_secret_key").as_deref());
            svc.guard_request(
                access,
                SystemTime::now(),
                request.get_param("critical").is_some(),
            )
        },
        |_| Response::empty_204(),
    )
    .into_result()
}

fn normalize_old_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("/old/") {
        format!("/{}", rest)
    } else {
        path.to_string()
    }
}

fn with_service<T, F, M>(state: &AppState, f: F, map_ok: M) -> Response
where
    F: FnOnce(
        &mut CringeService<LinuxShellBackend, FallbackLanguageDetector>,
    ) -> Result<T, CringeError>,
    M: FnOnce(T) -> Response,
{
    with_service_mut(state, f, map_ok)
}

fn with_service_mut<T, F, M>(state: &AppState, f: F, map_ok: M) -> Response
where
    F: FnOnce(
        &mut CringeService<LinuxShellBackend, FallbackLanguageDetector>,
    ) -> Result<T, CringeError>,
    M: FnOnce(T) -> Response,
{
    let mut svc = match state.service.lock() {
        Ok(guard) => guard,
        Err(_) => return map_error(CringeError::Backend("state lock poisoned".to_string())),
    };

    match f(&mut svc) {
        Ok(value) => map_ok(value),
        Err(e) => map_error(e),
    }
}

fn map_error(err: CringeError) -> Response {
    match err {
        CringeError::Forbidden(msg) => Response::text(msg).with_status_code(418),
        CringeError::InvalidInput(msg) => Response::text(msg).with_status_code(400),
        CringeError::Backend(msg) => Response::text(msg).with_status_code(500),
    }
}

fn spawn_audio_worker(
    backend: LinuxShellBackend,
    config: AppConfig,
    audio_rx: Receiver<AudioCommand>,
) {
    thread::spawn(move || {
        while let Ok(cmd) = audio_rx.recv() {
            let result = match cmd {
                AudioCommand::Speak {
                    saying,
                    lang_override,
                } => queue_speak(&backend, &config, &saying, lang_override.as_deref()),
                AudioCommand::PlayFile {
                    category,
                    filename_no_ext,
                } => backend.play_file(&category, &filename_no_ext),
            };

            if let Err(err) = result {
                eprintln!("audio worker command failed: {}", err);
            }
        }
    });
}

fn queue_speak(
    backend: &LinuxShellBackend,
    config: &AppConfig,
    saying: &str,
    lang_override: Option<&str>,
) -> Result<(), String> {
    let saying_sane = sanitize(saying);
    let sentences = smart_split(&saying_sane, config.max_api_str_len);
    let selected_lang = match lang_override {
        Some(lang) => sanitize(lang),
        None => "en".to_string(),
    };

    for sentence in sentences.into_iter().take(config.max_sentence_len) {
        backend.speak(&sentence, &selected_lang)?;
    }
    Ok(())
}

fn enqueue_audio(state: &AppState, cmd: AudioCommand, ok_message: String) -> Response {
    match state.queue_policy {
        QueuePolicy::DropNew => match state.audio_tx.try_send(cmd) {
            Ok(()) => Response::text(ok_message),
            Err(TrySendError::Full(_)) => {
                Response::text("audio queue full, dropping request").with_status_code(429)
            }
            Err(TrySendError::Disconnected(_)) => {
                map_error(CringeError::Backend("audio queue disconnected".to_string()))
            }
        },
        QueuePolicy::Block => match state.audio_tx.send(cmd) {
            Ok(()) => Response::text(ok_message),
            Err(_) => map_error(CringeError::Backend("audio queue disconnected".to_string())),
        },
    }
}

fn notify_lcd_api_request(state: &AppState, path: &str) {
    let message = fit_lcd_text(&format!("API {}", path), 31);
    match state.display_tx.try_send(LcdMessage::Row0(message)) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {}
        Err(TrySendError::Disconnected(_)) => {
            eprintln!("lcd display worker disconnected");
        }
    }
}

fn notify_lcd_volume(state: &AppState, volume: u8) {
    match state
        .display_tx
        .try_send(LcdMessage::Row1(volume_display_text(volume)))
    {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {}
        Err(TrySendError::Disconnected(_)) => {
            eprintln!("lcd display worker disconnected");
        }
    }
}

fn fit_lcd_text(text: &str, max_len: usize) -> String {
    text.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max_len)
        .collect()
}

fn volume_display_text(volume: u8) -> String {
    fit_lcd_text(&format!("Volume: {}%", volume), 31)
}

fn spawn_lcd_display_worker(
    serial_path: String,
    display_rx: Receiver<LcdMessage>,
    service: SharedService,
    volume_tx: Sender<u8>,
    volume_step: i16,
) {
    thread::spawn(move || {
        if serial_path.trim().is_empty() {
            while display_rx.recv().is_ok() {}
            return;
        }

        let mut serial: Option<File> = None;
        let mut serial_buf = Vec::new();
        let mut encoder_volume_target: Option<u8> = None;

        loop {
            if serial.is_none() {
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NOCTTY)
                    .open(&serial_path)
                {
                    Ok(mut file) => {
                        if let Err(err) = unassert_boot_lines(&file) {
                            eprintln!("lcd serial boot-line clear failed: {}", err);
                        }
                        if let Err(err) = configure_lcd_serial(&file) {
                            eprintln!("lcd serial configure failed: {}", err);
                        }
                        if let Err(err) = unassert_boot_lines(&file) {
                            eprintln!("lcd serial boot-line clear failed: {}", err);
                        }
                        if let Err(err) = read_lcd_heartbeat(&mut file, Duration::from_secs(4)) {
                            eprintln!("lcd firmware heartbeat not observed: {}", err);
                        }
                        thread::sleep(Duration::from_secs(2));
                        if let Err(err) = set_fd_nonblocking(&file, true) {
                            eprintln!("lcd serial nonblocking setup failed: {}", err);
                        }
                        if let Ok(volume) = current_volume(&service) {
                            encoder_volume_target = Some(volume);
                            if let Err(err) = write_lcd_message(
                                &mut file,
                                &LcdMessage::Row1(volume_display_text(volume)),
                            ) {
                                eprintln!("lcd serial write failed: {}", err);
                            }
                        }
                        serial = Some(file);
                    }
                    Err(err) => {
                        eprintln!("lcd serial open {} failed: {}", serial_path, err);
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                }
            }

            if let Some(file) = serial.as_mut() {
                let mut reopen_serial = false;

                loop {
                    match display_rx.try_recv() {
                        Ok(message) => {
                            if let Err(err) = write_lcd_message(file, &message) {
                                eprintln!("lcd serial write failed: {}", err);
                                reopen_serial = true;
                                break;
                            }
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break,
                    }
                }

                if !reopen_serial {
                    if let Err(err) = read_lcd_serial(
                        file,
                        &mut serial_buf,
                        &service,
                        &volume_tx,
                        volume_step,
                        &mut encoder_volume_target,
                    ) {
                        eprintln!("lcd serial read failed: {}", err);
                        reopen_serial = true;
                    }
                }

                if reopen_serial {
                    serial = None;
                    serial_buf.clear();
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

fn write_lcd_message(file: &mut File, message: &LcdMessage) -> Result<(), std::io::Error> {
    match message {
        LcdMessage::Row0(text) => file.write_all(format!("{}\r", text).as_bytes())?,
        LcdMessage::Row1(text) => file.write_all(format!("{}\t", text).as_bytes())?,
    }
    file.flush()
}

fn read_lcd_serial(
    file: &mut File,
    serial_buf: &mut Vec<u8>,
    service: &SharedService,
    volume_tx: &Sender<u8>,
    volume_step: i16,
    encoder_volume_target: &mut Option<u8>,
) -> Result<(), String> {
    let mut buf = [0u8; 256];

    loop {
        match file.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                serial_buf.extend_from_slice(&buf[..n]);
                while let Some(pos) = serial_buf.iter().position(|b| *b == b'\n' || *b == b'\r') {
                    let raw_line: Vec<u8> = serial_buf.drain(..=pos).collect();
                    let line = String::from_utf8_lossy(&raw_line);
                    handle_lcd_serial_line(
                        file,
                        line.trim(),
                        service,
                        volume_tx,
                        volume_step,
                        encoder_volume_target,
                    )?;
                }

                if serial_buf.len() > 512 {
                    serial_buf.clear();
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(err) => return Err(err.to_string()),
        }
    }
}

fn handle_lcd_serial_line(
    file: &mut File,
    line: &str,
    service: &SharedService,
    volume_tx: &Sender<u8>,
    volume_step: i16,
    encoder_volume_target: &mut Option<u8>,
) -> Result<(), String> {
    let delta = match line {
        "ENC:1" => volume_step,
        "ENC:-1" => -volume_step,
        _ => return Ok(()),
    };

    let volume = adjust_volume_target(service, encoder_volume_target, delta)?;
    if let Err(err) = volume_tx.send(volume) {
        eprintln!("volume persist worker unavailable: {}", err);
    }
    write_lcd_message(file, &LcdMessage::Row1(volume_display_text(volume)))
        .map_err(|e| e.to_string())?;
    println!("encoder volume set: {}%", volume);
    Ok(())
}

fn current_volume(service: &SharedService) -> Result<u8, String> {
    let svc = service
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
    svc.get_volume().map_err(|e| format!("{:?}", e))
}

fn adjust_volume_target(
    service: &SharedService,
    volume_target: &mut Option<u8>,
    delta: i16,
) -> Result<u8, String> {
    let svc = service
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
    let current = match *volume_target {
        Some(volume) => volume,
        None => svc.get_volume().map_err(|e| format!("{:?}", e))?,
    };
    let next = (current as i16 + delta).clamp(0, 100) as u8;
    svc.set_volume(next).map_err(|e| format!("{:?}", e))?;
    *volume_target = Some(next);
    Ok(next)
}

fn unassert_boot_lines(file: &File) -> Result<(), String> {
    let mut flags = libc::TIOCM_RTS | libc::TIOCM_DTR;
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), libc::TIOCMBIC, &mut flags) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

fn read_lcd_heartbeat(file: &mut File, timeout: Duration) -> Result<(), String> {
    let original_flags = set_fd_nonblocking(file, true)?;
    let deadline = Instant::now() + timeout;
    let mut seen = Vec::new();
    let mut buf = [0u8; 256];

    while Instant::now() < deadline {
        match file.read(&mut buf) {
            Ok(0) => thread::sleep(Duration::from_millis(100)),
            Ok(n) => {
                seen.extend_from_slice(&buf[..n]);
                if seen
                    .windows(b"LCD_FW_ALIVE".len())
                    .any(|w| w == b"LCD_FW_ALIVE")
                {
                    set_fd_flags(file, original_flags)?;
                    println!("lcd firmware heartbeat observed");
                    return Ok(());
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                let _ = set_fd_flags(file, original_flags);
                return Err(err.to_string());
            }
        }
    }

    set_fd_flags(file, original_flags)?;
    Err("timeout waiting for LCD_FW_ALIVE".to_string())
}

fn set_fd_nonblocking(file: &File, enabled: bool) -> Result<i32, String> {
    let fd = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(format!(
            "F_GETFL failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let updated = if enabled {
        flags | libc::O_NONBLOCK
    } else {
        flags & !libc::O_NONBLOCK
    };
    set_fd_flags(file, updated)?;
    Ok(flags)
}

fn set_fd_flags(file: &File, flags: i32) -> Result<(), String> {
    let rc = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags) };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!(
            "F_SETFL failed: {}",
            std::io::Error::last_os_error()
        ))
    }
}

fn configure_lcd_serial(file: &File) -> Result<(), String> {
    let fd = file.as_raw_fd();
    let mut termios = unsafe { mem::zeroed::<libc::termios>() };

    if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
        return Err(format!(
            "tcgetattr failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    unsafe { libc::cfmakeraw(&mut termios) };
    if unsafe { libc::cfsetispeed(&mut termios, libc::B115200) } != 0 {
        return Err(format!(
            "cfsetispeed failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    if unsafe { libc::cfsetospeed(&mut termios, libc::B115200) } != 0 {
        return Err(format!(
            "cfsetospeed failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    termios.c_cflag |= libc::CLOCAL | libc::CREAD;
    termios.c_cflag &= !libc::HUPCL;

    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) } != 0 {
        return Err(format!(
            "tcsetattr failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    Ok(())
}

fn queue_policy_name(policy: QueuePolicy) -> &'static str {
    match policy {
        QueuePolicy::DropNew => "drop_new",
        QueuePolicy::Block => "block",
    }
}

fn load_persisted_volume(path: &PathBuf) -> Option<u8> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            eprintln!(
                "failed to read persisted volume {}: {}",
                path.display(),
                err
            );
            return None;
        }
    };

    match raw.trim().parse::<u8>() {
        Ok(volume) if volume <= 100 => Some(volume),
        Ok(volume) => {
            eprintln!(
                "ignoring persisted volume outside 0-100 range: {} in {}",
                volume,
                path.display()
            );
            None
        }
        Err(err) => {
            eprintln!(
                "ignoring invalid persisted volume in {}: {}",
                path.display(),
                err
            );
            None
        }
    }
}

fn spawn_volume_persist_worker(path: PathBuf, volume_rx: Receiver<u8>) {
    thread::spawn(move || {
        let mut last_write: Option<Instant> = None;
        let mut last_persisted = load_persisted_volume(&path);

        while let Ok(mut latest) = volume_rx.recv() {
            loop {
                let wait = last_write
                    .and_then(|last| VOLUME_PERSIST_DEBOUNCE.checked_sub(last.elapsed()))
                    .unwrap_or_default();

                if wait.is_zero() {
                    break;
                }

                match volume_rx.recv_timeout(wait) {
                    Ok(next) => latest = next,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            loop {
                match volume_rx.try_recv() {
                    Ok(next) => latest = next,
                    Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
                }
            }

            if last_persisted == Some(latest) {
                continue;
            }

            if let Err(err) = persist_volume(&path, latest) {
                eprintln!("failed to persist volume {}%: {}", latest, err);
            } else {
                last_write = Some(Instant::now());
                last_persisted = Some(latest);
            }
        }
    });
}

fn persist_volume(path: &PathBuf, volume: u8) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create state dir failed: {}", e))?;
    }

    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, format!("{}\n", volume)).map_err(|e| format!("write failed: {}", e))?;
    fs::rename(&tmp_path, path).map_err(|e| format!("rename failed: {}", e))
}

fn serve_file(path: &str, content_type: &'static str) -> Response {
    match fs::read(path) {
        Ok(bytes) => Response::from_data(content_type, bytes),
        Err(_) => Response::empty_404(),
    }
}

fn handle_uploader(state: &AppState, request: &Request) -> Response {
    let data = match rouille::post_input!(request, {
        file: rouille::input::post::BufferedFile,
    }) {
        Ok(data) => data,
        Err(e) => {
            return Response::text(format!("invalid upload payload: {:?}", e))
                .with_status_code(400);
        }
    };

    let upload_dir = format!("{}/audio_files/recent_upload", state.root);
    if let Err(e) = fs::create_dir_all(&upload_dir) {
        return map_error(CringeError::Backend(format!(
            "upload dir create failed: {}",
            e
        )));
    }

    let target = format!("{}/upload.mp3", upload_dir);
    if let Err(e) = fs::write(&target, data.file.data) {
        return map_error(CringeError::Backend(format!("upload write failed: {}", e)));
    }

    enqueue_audio(
        state,
        AudioCommand::PlayFile {
            category: "recent_upload".to_string(),
            filename_no_ext: "upload".to_string(),
        },
        "file uploaded".to_string(),
    )
}

fn mime_from_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}

fn is_arm() -> bool {
    std::env::consts::ARCH.contains("arm") || std::env::consts::ARCH.contains("aarch")
}

trait IntoGuardResult {
    fn into_result(self) -> Result<(), Response>;
}

impl IntoGuardResult for Response {
    fn into_result(self) -> Result<(), Response> {
        if self.status_code == 204 {
            Ok(())
        } else {
            Err(self)
        }
    }
}
