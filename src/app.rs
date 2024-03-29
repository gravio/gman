use std::path::PathBuf;

#[allow(unused_imports)]
use log::Log;

pub const APP_FOLDER_NAME: &'static str = "gman_5a8f853f-d7e7-4a83-aa21-6ed0585b0c40";

pub const CLIENT_CONFIG_FILE_NAME: &'static str = "./gman_config_client.json5";

static INITD: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

pub fn get_app_temp_directory() -> PathBuf {
    std::env::temp_dir().join(APP_FOLDER_NAME)
}

/// Disables global logging, and returns the last level used
pub fn disable_logging() -> log::LevelFilter {
    let last_level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);
    last_level
}

pub fn enable_logging(max_level: log::LevelFilter) {
    log::set_max_level(max_level);
}

pub fn init_logging(max_level: Option<log::LevelFilter>) {
    let mut r = INITD.lock().unwrap();
    if !(*r) {
        simple_logger::SimpleLogger::new().env().init().unwrap();
        if let Some(ll) = max_level {
            enable_logging(ll);
        }
        *r = true;
    }
}
