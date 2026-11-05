mod backup_restore;
mod config;
mod consts;
mod runner;

use crate::backup_restore::restore_backup;
use clap::Parser;
use std::path::PathBuf;

const FAULT_LOG_FILENAME: &str = "home-assistant.log.fault";

/// Validate that Home Assistant is running in a supported operating system.
fn validate_os() {
    if !cfg!(target_family = "unix") {
        eprintln!("Home Assistant only supports Linux, OSX and Windows using WSL");
        std::process::exit(1);
    }
}

/// Validate the configuration directory.
fn ensure_config_path(config_dir: &PathBuf) {
    if !config_dir.is_dir() {
        if config_dir.clone() != config::get_default_config_dir() {
            eprintln!(
                "Fatal Error: Specified configuration directory {} {}",
                config_dir.clone().into_string().unwrap(),
                if config_dir.exists() {
                    "is not a directory"
                } else {
                    "does not exist"
                }
            );
            std::process::exit(1);
        }

        std::fs::create_dir_all(&config_dir).unwrap_or_else(|error| {
            eprintln!(
                "Fatal Error: Unable to create default configuration directory {}: {}",
                config_dir.clone().into_string().unwrap(),
                error
            );
            std::process::exit(1);
        })
    }
}

#[derive(Parser)]
#[command(
    name = "Home Assistant Core (Rust)",
    version,
    about = "Home Assistant: Observe, Control, Automate."
)]
struct Cli {
    #[arg(short = 'c', long)]
    /// Directory that contains the Home Assistant configuration
    config: Option<PathBuf>,
    #[arg(long)]
    /// Start Home Assistant in recovery mode
    recovery_mode: bool,
    #[arg(long)]
    /// Start Home Assistant in debug mode
    debug: bool,
    #[arg(long)]
    /// Open the webinterface in a browser
    open_ui: bool,
    #[arg(short = 'v', long)]
    /// Enable verbose logging to file.
    verbose: bool,
    #[arg(long)]
    /// Enables daily log retention and keeps up to the specified days
    log_rotate_days: Option<u32>,
    #[arg(long)]
    /// Log file to write to. If not set, CONFIG/home-assistant.log is used
    log_file: Option<PathBuf>,
    #[arg(long)]
    /// Disable color logs
    log_no_color: bool,
    #[arg(long)]
    /// Skips validation of operating system
    ignore_os_check: bool,
}

fn main() {
    let cli = Cli::parse();

    if !cli.ignore_os_check {
        validate_os()
    }

    let config_dir = std::env::current_dir()
        .unwrap()
        .join(cli.config.unwrap_or_else(config::get_default_config_dir));

    if restore_backup(&config_dir) {
        std::process::exit(consts::RESTART_EXIT_CODE.into())
    }

    ensure_config_path(&config_dir);

    // Ensure only one instance runs per config directory
    let single_execution_lock = runner::ensure_single_execution(&config_dir);
    // Check if another instance is already running
    if let Some(exit_code) = single_execution_lock.exit_code {
        std::process::exit(exit_code)
    }

    let safe_mode = config::safe_mode_enabled(&config_dir);

    let fault_file_path = config_dir.join(FAULT_LOG_FILENAME);
    let runtime_conf = runner::RuntimeConfig {
        config_dir,
        verbose: cli.verbose,
        log_rotate_days: cli.log_rotate_days,
        log_file: cli.log_file,
        log_no_color: cli.log_no_color,
        recovery_mode: cli.recovery_mode,
        debug: cli.debug,
        open_ui: cli.open_ui,
        safe_mode,
    };


}
