use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// Lock file configuration
const LOCK_FILE_NAME: &str = ".ha_run.lock";
const LOCK_FILE_VERSION: u8 = 1;

#[derive(Default, Debug)]
/// Context object for single execution lock.
pub struct SingleExecutionLock {
    pub exit_code: Option<i32>,
    _lock_file: Option<File>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InstanceInfo {
    pid: u32,
    version: u8,
    ha_version: String,
    start_ts: u64,
}

impl InstanceInfo {
    fn new() -> Self {
        Self {
            pid: std::process::id(),
            version: LOCK_FILE_VERSION,
            ha_version: env!("CARGO_PKG_VERSION").to_string(),
            start_ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Write current instance information to the lock file.
///
/// Args:
/// - `lock_file`: The open lock file handle.
fn write_lock_info(lock_file: &mut File) {
    lock_file.seek_relative(0).unwrap();

    let instance_info = InstanceInfo::new();
    let json = serde_json::to_string_pretty(&instance_info).unwrap();
    lock_file.write_all(json.as_bytes()).unwrap();
}

/// Report that another instance is already running.
///
/// Attempts to read the lock file to provide details about the running instance.
fn report_existing_instance(lock_file_path: &PathBuf, config_dir: &PathBuf) {
    let mut error_msg: Vec<String> = Vec::new();
    error_msg.push("Error: Another Home Assistant instance is already running!".into());

    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_file_path);

    match lock_file {
        Ok(mut lock_file) => {
            let mut content = String::new();
            match lock_file.read_to_string(&mut content) {
                Ok(_) => {
                    let existing_info: InstanceInfo = serde_json::from_str(&content).unwrap();
                    error_msg.push(format!("  PID: {}", existing_info.pid));
                    error_msg.push(format!("  Version: {}", existing_info.ha_version));
                    error_msg.push(format!("  Started: {}", existing_info.start_ts));
                }
                Err(_) => error_msg.push("  Unable to read lock file details.".into()),
            }
        }
        Err(error) => error_msg.push(format!("  Unable to read lock file details: {error}")),
    }

    error_msg.push(format!(
        "  Config directory: {}",
        config_dir.clone().into_string().unwrap()
    ));
    error_msg.push("".into());
    error_msg.push("Please stop the existing instance before starting a new one.".into());

    eprintln!("{}", error_msg.join("\n"));
}

/// Ensure only one Home Assistant instance runs per config directory.
///
/// Uses file locking to prevent multiple instances from running with the
/// same configuration directory, which can cause data corruption.
///
/// Returns a context object with `exit_code` attribute that will be set
/// if another instance is already running.
pub fn ensure_single_execution(config_dir: &PathBuf) -> SingleExecutionLock {
    let lock_file_path = config_dir.join(LOCK_FILE_NAME);
    let mut lock_context = SingleExecutionLock::default();

    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_file_path);

    match lock_file {
        Ok(mut lock_file) => {
            match lock_file.try_lock_exclusive() {
                Ok(_) => {
                    write_lock_info(&mut lock_file);
                    lock_context._lock_file = Some(lock_file);
                    lock_context
                }
                Err(_) => {
                    // Another instance is already running
                    report_existing_instance(&lock_file_path, config_dir);
                    lock_context.exit_code = Some(1);
                    lock_context
                }
            }
        }
        Err(_) => {
            lock_context.exit_code = Some(1);
            lock_context
        }
    }
}

/// Struct to hold the information for running Home Assistant.
pub struct RuntimeConfig {
    pub config_dir: PathBuf,
    pub recovery_mode: bool,
    pub verbose: bool,
    pub log_rotate_days: Option<u32>,
    pub log_file: Option<PathBuf>,
    pub log_no_color: bool,
    pub debug: bool,
    pub open_ui: bool,
    pub safe_mode: bool,
}
