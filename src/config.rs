use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = ".homeassistant";

const SAFE_MODE_FILENAME: &str = "safe-mode";

pub fn get_default_config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(CONFIG_DIR_NAME)
}

/// Return if safe mode is enabled.
///
/// If safe mode is enabled, the safe mode file will be removed.
pub fn safe_mode_enabled(config_dir: &PathBuf) -> bool {
    let safe_mode_path = config_dir.join(SAFE_MODE_FILENAME);
    let safe_mode = safe_mode_path.exists();
    if safe_mode {
        fs::remove_file(&safe_mode_path).unwrap();
    }
    safe_mode
}
