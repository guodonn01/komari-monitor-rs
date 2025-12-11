use log::info;
use miniserde::{Deserialize, Serialize};
use palc::{Parser, ValueEnum};
use std::path::PathBuf;
use std::env;

#[derive(Parser, Debug, Clone)]
#[command(
    version,
    long_about = "komari-monitor-rs is a third-party high-performance monitoring agent for the komari monitoring service.",
    after_long_help = "Must set --http-server / --token\n--ip-provider accepts cloudflare / ipinfo\n--log-level accepts error, warn, info, debug, trace\n\nThis Agent is open-sourced on Github, powered by powerful Rust. Love from Komari"
)]
pub struct Args {
    // Main
    /// Set Main Server Http Address
    #[arg(long)]
    pub http_server: Option<String>,

    /// Set Main Server WebSocket Address
    #[arg(long)]
    pub ws_server: Option<String>,

    /// Set Token
    #[arg(short, long, allow_hyphen_values = true)]
    pub token: Option<String>,

    /// Set Fake Multiplier
    #[arg(short, long, default_value_t = 1.0)]
    pub fake: f64,

    /// Enable TLS (default disabled)
    #[arg(long, default_value_t = false)]
    pub tls: bool,

    /// Ignore Certificate Verification
    #[arg(long, default_value_t = false)]
    pub ignore_unsafe_cert: bool,

    /// Dry Run
    #[arg(short, long, default_value_t = false)]
    pub dry_run: bool,

    /// Set Log Level (Enable Debug or Trace for issue reporting)
    #[arg(long, default_value_t = log_level())]
    pub log_level: LogLevel,

    // Other
    /// Public IP Provider
    #[arg(long, default_value_t=ip_provider())]
    pub ip_provider: IpProvider,

    /// Set Real-Time Info Upload Interval (ms)
    #[arg(long, default_value_t = 1000)]
    pub realtime_info_interval: u64,

    // Network
    /// Disable Network Statistics
    #[arg(long, default_value_t = false)]
    pub disable_network_statistics: bool,

    /// Network Statistics Duration (s)
    #[arg(long, default_value_t = 864000)]
    pub network_duration: u32,

    /// Network Statistics Interval (s)
    #[arg(long, default_value_t = 10)]
    pub network_interval: u32,

    /// Network Statistics Save to Disk Interval Count (s)
    #[arg(long, default_value_t = 10)]
    pub network_interval_number: u32,

    /// Network Statistics Save Path
    #[arg(long)]
    pub network_save_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkConfig {
    pub disable_network_statistics: bool,
    pub network_duration: u32,
    pub network_interval: u32,
    pub network_interval_number: u32,
    pub network_save_path: String,
}

impl Args {
    pub fn par() -> Self {
        let args = Self::parse();
        unsafe {
            crate::get_info::network::DURATION = args.realtime_info_interval as f64;
        }
        args
    }
    pub fn network_config(&self) -> NetworkConfig {
        NetworkConfig {
            disable_network_statistics: self.disable_network_statistics,
            network_duration: self.network_duration,
            network_interval: self.network_interval,
            network_interval_number: self.network_interval_number,
            network_save_path: {
                if self.network_save_path.is_none() {
                    if cfg!(windows) {
                        PathBuf::from(r"C:\komari-network.conf")
                            .to_string_lossy()
                            .to_string()
                    } else {
                        let is_root = env::var("EUID")
                            .unwrap_or("999".to_string())
                            .parse::<i32>()
                            .unwrap_or(999)
                            == 0
                            || env::var("UID")
                                .unwrap_or("999".to_string())
                                .parse::<i32>()
                                .unwrap_or(999)
                                == 0;
                        let path = if is_root {
                            PathBuf::from("/etc/komari-network.conf")
                                .to_string_lossy()
                                .to_string()
                        } else {
                            PathBuf::from("/opt/komari")
                                .join(".config/komari-network.conf")
                                .to_string_lossy()
                                .to_string()
                        };
                        info!(
                            "Automatically determined Network Config save path: {}",
                            path
                        );
                        path
                    }
                } else {
                    let path = PathBuf::from(self.network_save_path.as_ref().unwrap())
                        .to_string_lossy()
                        .to_string();
                    info!("Using specified Network Config save path: {}", path);
                    path
                }
            },
        }
    }
}

// Default Settings

fn ip_provider() -> IpProvider {
    IpProvider::Ipinfo
}

#[derive(Debug, Clone, ValueEnum)]
pub enum IpProvider {
    Cloudflare,
    Ipinfo,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

fn log_level() -> LogLevel {
    LogLevel::Info
}
