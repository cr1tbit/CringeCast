use std::collections::BTreeMap;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub max_sentence_len: usize,
    pub max_api_str_len: usize,
    pub privileged_volume: u8,
    pub super_secret_key: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_sentence_len: 16,
            max_api_str_len: 200,
            privileged_volume: 80,
            super_secret_key: "change-me".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Privileged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeapotRequest {
    On,
    Off,
    Permanent,
}

#[derive(Debug)]
pub enum CringeError {
    Forbidden(String),
    InvalidInput(String),
    Backend(String),
}

pub trait AudioBackend {
    fn speak(&self, text: &str, lang: &str) -> Result<(), String>;
    fn play_file(&self, category: &str, filename_no_ext: &str) -> Result<(), String>;
    fn stop_all(&self) -> Result<(), String>;
    fn get_volume(&self) -> Result<u8, String>;
    fn set_volume(&self, volume_percent: u8) -> Result<(), String>;
    fn list_audio_files(&self) -> Result<BTreeMap<String, Vec<String>>, String>;
}

pub trait LanguageDetector {
    fn detect(&self, text: &str) -> String;
}

pub struct FallbackLanguageDetector;

impl LanguageDetector for FallbackLanguageDetector {
    fn detect(&self, _text: &str) -> String {
        "en".to_string()
    }
}

#[derive(Clone, Debug)]
pub struct TeapotMode {
    unmute_at: Option<SystemTime>,
}

impl Default for TeapotMode {
    fn default() -> Self {
        Self {
            unmute_at: Some(UNIX_EPOCH),
        }
    }
}

impl TeapotMode {
    pub fn in_teapot_mode(&self, now: SystemTime) -> bool {
        match self.unmute_at {
            Some(deadline) => now < deadline,
            None => false,
        }
    }

    pub fn get_remaining(&self, now: SystemTime) -> Duration {
        match self.unmute_at {
            Some(deadline) if now < deadline => deadline.duration_since(now).unwrap_or_default(),
            _ => Duration::from_secs(0),
        }
    }

    pub fn disable(&mut self) {
        self.unmute_at = None;
    }

    pub fn set(&mut self, now: SystemTime, permanent: bool) {
        if permanent {
            self.unmute_at = Some(now + Duration::from_secs(69 * 24 * 60 * 60));
            return;
        }

        let escalate = self
            .unmute_at
            .map(|prev| signed_diff_seconds(now, prev) < (20 * 60) as i64)
            .unwrap_or(false);

        self.unmute_at = if escalate {
            Some(now + Duration::from_secs(4 * 60 * 60))
        } else {
            Some(now + Duration::from_secs(15 * 60))
        };
    }
}

pub struct CringeService<B: AudioBackend, D: LanguageDetector> {
    pub config: AppConfig,
    pub teapot: TeapotMode,
    backend: B,
    detector: D,
}

impl<B: AudioBackend, D: LanguageDetector> CringeService<B, D> {
    pub fn new(config: AppConfig, backend: B, detector: D) -> Self {
        Self {
            config,
            teapot: TeapotMode::default(),
            backend,
            detector,
        }
    }

    pub fn resolve_access(&self, super_secret_key: Option<&str>) -> AccessLevel {
        match super_secret_key {
            Some(key) if key == self.config.super_secret_key => AccessLevel::Privileged,
            _ => AccessLevel::Public,
        }
    }

    pub fn guard_request(
        &mut self,
        access: AccessLevel,
        now: SystemTime,
        critical: bool,
    ) -> Result<(), CringeError> {
        if access == AccessLevel::Privileged && critical {
            self.backend
                .set_volume(self.config.privileged_volume)
                .map_err(CringeError::Backend)?;
        }

        if self.teapot.in_teapot_mode(now) && access != AccessLevel::Privileged {
            let left = self.teapot.get_remaining(now).as_secs();
            return Err(CringeError::Forbidden(format!(
                "I'm sorry, I'm a teapot for next {}s",
                left
            )));
        }

        Ok(())
    }

    pub fn teapot_control(
        &mut self,
        req: TeapotRequest,
        access: AccessLevel,
        now: SystemTime,
    ) -> Result<&'static str, CringeError> {
        match req {
            TeapotRequest::On => {
                self.teapot.set(now, false);
                Ok("Teapot mode set")
            }
            TeapotRequest::Off => {
                self.teapot.disable();
                Ok("Teapot mode disabled")
            }
            TeapotRequest::Permanent if access == AccessLevel::Privileged => {
                self.teapot.set(now, true);
                Ok("I'm permanently teapot now.")
            }
            TeapotRequest::Permanent => Err(CringeError::Forbidden(
                "Permanent teapot mode requires privileged access".to_string(),
            )),
        }
    }

    pub fn teapot_status(&self, now: SystemTime) -> bool {
        self.teapot.in_teapot_mode(now)
    }

    pub fn speak(&self, saying: &str, lang_override: Option<&str>) -> Result<(), CringeError> {
        let saying_sane = sanitize(saying);
        let sentences = smart_split(&saying_sane, self.config.max_api_str_len);
        let selected_lang = match lang_override {
            Some(lang) => sanitize(lang),
            None => self
                .detector
                .detect(sentences.first().map(String::as_str).unwrap_or("")),
        };

        for sentence in sentences.into_iter().take(self.config.max_sentence_len) {
            self.backend
                .speak(&sentence, &selected_lang)
                .map_err(CringeError::Backend)?;
        }
        Ok(())
    }

    pub fn play_file(&self, category: &str, filename_no_ext: &str) -> Result<(), CringeError> {
        self.backend
            .play_file(&sanitize(category), &sanitize(filename_no_ext))
            .map_err(CringeError::Backend)
    }

    pub fn stop(&self) -> Result<(), CringeError> {
        self.backend.stop_all().map_err(CringeError::Backend)
    }

    pub fn get_volume(&self) -> Result<u8, CringeError> {
        self.backend.get_volume().map_err(CringeError::Backend)
    }

    pub fn set_volume(&self, volume_percent: u8) -> Result<(), CringeError> {
        if volume_percent > 100 {
            return Err(CringeError::InvalidInput(format!(
                "invalid volume requested: {}%",
                volume_percent
            )));
        }
        self.backend
            .set_volume(volume_percent)
            .map_err(CringeError::Backend)
    }

    pub fn list_files(&self) -> Result<BTreeMap<String, Vec<String>>, CringeError> {
        self.backend.list_audio_files().map_err(CringeError::Backend)
    }
}

pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| {
            c.is_ascii_alphanumeric()
                || match c {
                    ',' | '.' | '?' | '!' | '\'' | '_' | '-' | ' ' | 'ż' | 'ź' | 'ć' | 'ń'
                    | 'ó' | 'ł' | 'ę' | 'ą' | 'ś' | 'Ż' | 'Ź' | 'Ć' | 'Ą' | 'Ś' | 'Ę'
                    | 'Ł' | 'Ó' | 'Ń' => true,
                    _ => false,
                }
                || is_cjk(*c)
        })
        .collect()
}

pub fn smart_split(s: &str, max_api_str_len: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in s.split(|c| c == '.' || c == '?' || c == '!') {
        if raw.len() <= max_api_str_len {
            out.push(raw.to_string());
        } else {
            out.push(raw[..max_api_str_len].to_string());
        }
    }
    out
}

fn is_cjk(c: char) -> bool {
    (c as u32) >= 0x4E00 && (c as u32) <= 0x9FFF
}

fn signed_diff_seconds(left: SystemTime, right: SystemTime) -> i64 {
    let left_s = unix_seconds(left);
    let right_s = unix_seconds(right);
    left_s - right_s
}

fn unix_seconds(ts: SystemTime) -> i64 {
    ts.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn run_shell_script(script: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("bash")
        .arg(script)
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute bash {}: {}", script, e))?;

    if !output.status.success() {
        return Err(format!(
            "script {} failed with status {}",
            script,
            output.status
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("stdout decode failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_dangerous_chars() {
        let out = sanitize("hello$() rm -rf /\\");
        assert_eq!(out, "hello rm -rf ");
    }

    #[test]
    fn split_over_limit() {
        let text = "a".repeat(210);
        let split = smart_split(&text, 200);
        assert_eq!(split[0].len(), 200);
    }

    #[test]
    fn teapot_escalates_on_repeat() {
        let mut tm = TeapotMode::default();
        let now = UNIX_EPOCH + Duration::from_secs(10_000);

        tm.set(now, false);
        let first = tm.get_remaining(now);
        assert_eq!(first, Duration::from_secs(15 * 60));

        tm.set(now + Duration::from_secs(5), false);
        let second = tm.get_remaining(now + Duration::from_secs(5));
        assert_eq!(second, Duration::from_secs(4 * 60 * 60));
    }
}
