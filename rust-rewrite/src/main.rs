use cringecast::core::{AppConfig, CringeError, CringeService, FallbackLanguageDetector, TeapotRequest};
use cringecast::platform::linux_shell::LinuxShellBackend;
use rouille::{Request, Response};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Clone)]
struct AppState {
    root: String,
    service: Arc<Mutex<CringeService<LinuxShellBackend, FallbackLanguageDetector>>>,
}

fn main() {
    let root = std::env::var("CRINGECAST_ROOT").unwrap_or_else(|_| ".".to_string());
    let port = std::env::var("CRINGECAST_PORT").unwrap_or_else(|_| "42069".to_string());
    let super_secret_key =
        std::env::var("CRINGECAST_SUPER_SECRET_KEY").unwrap_or_else(|_| "change-me".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let backend = LinuxShellBackend::new(
        format!("{}/CringeCast/shellscripts", root),
        format!("{}/audio_files", root),
        is_arm(),
    );
    let mut config = AppConfig::default();
    config.super_secret_key = super_secret_key;
    let service = CringeService::new(config, backend, FallbackLanguageDetector);

    let state = AppState {
        root: root,
        service: Arc::new(Mutex::new(service)),
    };

    println!("cringecast-server listening on {}", addr);
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
        return handle_uploader(state, request);
    }

    let segments: Vec<&str> = path_only.trim_start_matches('/').split('/').collect();

    if path_only == "/stop" {
        return with_service(state, |svc| svc.stop(), |()| Response::text("OK: Stopped"));
    }

    if path_only == "/vol" {
        return with_service(state, |svc| svc.get_volume(), |vol| Response::text(vol.to_string()));
    }

    if segments.len() == 2 && segments[0] == "vol" {
        let vol = segments[1].parse::<u8>();
        return match vol {
            Ok(v) => with_service(
                state,
                |svc| svc.set_volume(v),
                |()| Response::text(format!("volume set ok: {}%", v)),
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
        return with_service_mut(state, |svc| {
            let access = svc.resolve_access(request.get_param("super_secret_key").as_deref());
            let mode = match segments[1] {
                "on" => TeapotRequest::On,
                "off" => TeapotRequest::Off,
                "permanent" => TeapotRequest::Permanent,
                _ => return Err(CringeError::InvalidInput("Invalid request".to_string())),
            };
            svc.teapot_control(mode, access, SystemTime::now())
        }, |msg| Response::text(msg));
    }

    if segments.len() == 2 && segments[0] == "say" {
        return with_service(
            state,
            |svc| svc.speak(segments[1], Some("en")),
            |()| Response::text(format!("OK: {}", segments[1])),
        );
    }

    if segments.len() == 2 && segments[0] == "mow" {
        return with_service(
            state,
            |svc| svc.speak(segments[1], Some("pl")),
            |()| Response::text(format!("OK: {}", segments[1])),
        );
    }

    if segments.len() == 2 && segments[0] == "guess" {
        let lang = request.get_param("l");
        return with_service(
            state,
            |svc| svc.speak(segments[1], lang.as_ref().map(String::as_str)),
            |()| Response::text(format!("OK: {}, language is: {:?}", segments[1], lang)),
        );
    }

    if segments.len() == 3 && segments[0] == "play" {
        return with_service(
            state,
            |svc| svc.play_file(segments[1], segments[2]),
            |()| Response::text(format!("OK: {}/{}.mp3", segments[1], segments[2])),
        );
    }

    if segments.len() == 1 && !segments[0].is_empty() {
        let lang = request.get_param("l");
        return with_service(
            state,
            |svc| svc.speak(segments[0], lang.as_ref().map(String::as_str)),
            |()| Response::text(format!("OK: {}, language is: {:?}", segments[0], lang)),
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

fn with_service<T, F, M>(
    state: &AppState,
    f: F,
    map_ok: M,
) -> Response
where
    F: FnOnce(&mut CringeService<LinuxShellBackend, FallbackLanguageDetector>) -> Result<T, CringeError>,
    M: FnOnce(T) -> Response,
{
    with_service_mut(state, f, map_ok)
}

fn with_service_mut<T, F, M>(
    state: &AppState,
    f: F,
    map_ok: M,
) -> Response
where
    F: FnOnce(&mut CringeService<LinuxShellBackend, FallbackLanguageDetector>) -> Result<T, CringeError>,
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
            return Response::text(format!("invalid upload payload: {:?}", e)).with_status_code(400);
        }
    };

    let upload_dir = format!("{}/audio_files/recent_upload", state.root);
    if let Err(e) = fs::create_dir_all(&upload_dir) {
        return map_error(CringeError::Backend(format!("upload dir create failed: {}", e)));
    }

    let target = format!("{}/upload.mp3", upload_dir);
    if let Err(e) = fs::write(&target, data.file.data) {
        return map_error(CringeError::Backend(format!("upload write failed: {}", e)));
    }

    with_service(
        state,
        |svc| svc.play_file("recent_upload", "upload"),
        |()| Response::text("file uploaded"),
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
