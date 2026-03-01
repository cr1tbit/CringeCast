use crate::core::{AudioBackend, LanguageDetector};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Debug)]
pub struct MockState {
    pub spoken: Vec<(String, String)>,
    pub played: Vec<(String, String)>,
    pub stopped: usize,
    pub volume: u8,
    pub files: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Default)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_files(files: BTreeMap<String, Vec<String>>) -> Self {
        let backend = Self::default();
        backend.state.lock().expect("poisoned").files = files;
        backend
    }

    pub fn state(&self) -> Arc<Mutex<MockState>> {
        self.state.clone()
    }
}

impl AudioBackend for MockBackend {
    fn speak(&self, text: &str, lang: &str) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .spoken
            .push((text.to_string(), lang.to_string()));
        Ok(())
    }

    fn play_file(&self, category: &str, filename_no_ext: &str) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .played
            .push((category.to_string(), filename_no_ext.to_string()));
        Ok(())
    }

    fn stop_all(&self) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .stopped += 1;
        Ok(())
    }

    fn get_volume(&self) -> Result<u8, String> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .volume)
    }

    fn set_volume(&self, volume_percent: u8) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .volume = volume_percent;
        Ok(())
    }

    fn list_audio_files(&self) -> Result<BTreeMap<String, Vec<String>>, String> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "mutex poisoned".to_string())?
            .files
            .clone())
    }
}

pub struct FixedLanguageDetector {
    lang: String,
}

impl FixedLanguageDetector {
    pub fn new(lang: impl Into<String>) -> Self {
        Self { lang: lang.into() }
    }
}

impl LanguageDetector for FixedLanguageDetector {
    fn detect(&self, _text: &str) -> String {
        self.lang.clone()
    }
}
