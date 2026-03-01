use crate::core::{run_shell_script, AudioBackend};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct LinuxShellBackend {
    pub shell_scripts_dir: PathBuf,
    pub audio_root: PathBuf,
    pub is_arm: bool,
}

impl LinuxShellBackend {
    pub fn new(shell_scripts_dir: impl Into<PathBuf>, audio_root: impl Into<PathBuf>, is_arm: bool) -> Self {
        Self {
            shell_scripts_dir: shell_scripts_dir.into(),
            audio_root: audio_root.into(),
            is_arm,
        }
    }

    fn script_path(&self, name: &str) -> String {
        self.shell_scripts_dir.join(name).to_string_lossy().to_string()
    }
}

impl AudioBackend for LinuxShellBackend {
    fn speak(&self, text: &str, lang: &str) -> Result<(), String> {
        let script = if self.is_arm { "speak_arm.sh" } else { "speak.sh" };
        run_shell_script(&self.script_path(script), &[text, lang]).map(|_| ())
    }

    fn play_file(&self, category: &str, filename_no_ext: &str) -> Result<(), String> {
        let script = if self.is_arm { "play_arm.sh" } else { "play.sh" };
        let file = self
            .audio_root
            .join(category)
            .join(format!("{}.mp3", filename_no_ext));
        run_shell_script(&self.script_path(script), &[file.to_string_lossy().as_ref()]).map(|_| ())
    }

    fn stop_all(&self) -> Result<(), String> {
        run_shell_script(&self.script_path("kill.sh"), &[]).map(|_| ())
    }

    fn get_volume(&self) -> Result<u8, String> {
        let script = if self.is_arm {
            "get_vol_arm.sh"
        } else {
            "get_vol.sh"
        };
        let out = run_shell_script(&self.script_path(script), &[])?;
        out.trim()
            .parse::<u8>()
            .map_err(|e| format!("invalid volume output '{}': {}", out, e))
    }

    fn set_volume(&self, volume_percent: u8) -> Result<(), String> {
        let script = if self.is_arm {
            "set_vol_arm.sh"
        } else {
            "set_vol.sh"
        };

        run_shell_script(&self.script_path(script), &[&volume_percent.to_string()]).map(|_| ())
    }

    fn list_audio_files(&self) -> Result<BTreeMap<String, Vec<String>>, String> {
        let mut out = BTreeMap::new();
        for category in fs::read_dir(&self.audio_root).map_err(io_to_string)? {
            let category = category.map_err(io_to_string)?;
            let path = category.path();
            if !path.is_dir() {
                continue;
            }

            let name = category.file_name().to_string_lossy().to_string();
            let mut files = Vec::new();

            for item in fs::read_dir(path).map_err(io_to_string)? {
                let item = item.map_err(io_to_string)?;
                let file_path = item.path();
                if file_path
                    .extension()
                    .map(|ext| ext == "mp3")
                    .unwrap_or(false)
                {
                    files.push(strip_mp3_ext(&file_path));
                }
            }

            files.sort();
            out.insert(name, files);
        }

        Ok(out)
    }
}

fn strip_mp3_ext(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn io_to_string(err: std::io::Error) -> String {
    err.to_string()
}
