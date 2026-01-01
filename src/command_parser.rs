use log::info;
use miniserde::{Deserialize, Serialize};
use palc::{Parser, ValueEnum};
use std::fmt::Display;
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

    /// Disable Windows Toast Notification (Only Windows)
    #[arg(long, default_value_t = false)]
    pub disable_toast_notify: bool,

    // Network
    /// Disable Network Statistics
    #[arg(long, default_value_t = false)]
    pub disable_network_statistics: bool,

    #[doc = "Network statistics calculation mode.
    \t  'fixed' is based on a fixed duration, such as 10 days
    \t  'natural' is based on natural datetime"]
    #[arg(long, value_enum, default_value_t = network_statistics_mode())]
    pub network_statistics_mode: NetworkStatisticsMode,

    /// Network Statistics Save Path
    #[arg(long)]
    pub network_save_path: Option<String>,

    /// Network Statistics Save Interval (s)
    #[arg(long, default_value_t = 10)]
    pub network_interval: u32,

    #[doc = "For 'fixed' mode only
    \t  Duration for one cycle of network statistics in seconds."]
    #[arg(long, default_value_t = 864000)] // 10 days
    pub network_duration: u32,
    
    /// Number of intervals to save network statistics to disk.
    #[arg(long, default_value_t = 6)]
    pub network_interval_number: u32,

    /// Network statistics reset period, for 'natural' mode only.
    #[arg(long, value_enum, default_value_t = traffic_period())]
    pub traffic_period: TrafficPeriod,

    #[doc = "Network statistics reset day, for 'natural' mode only.
    \t    For 'week', accepts 1-7 (Mon-Sun) or names like 'mon', 'tue'.
    \t    For 'month', accepts a day number like 1-31.
    \t    For 'year', accepts a date in 'MM/DD' format, e.g., '12/31'."]
    #[arg(long, default_value_t = String::from("1"))]
    pub traffic_reset_day: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkConfig {
    pub disable_network_statistics: bool,
    pub network_interval: u32,
    pub network_save_path: String,
    pub traffic_period: TrafficPeriod,
    pub traffic_reset_day: String,
    pub network_statistics_mode: NetworkStatisticsMode,
    pub network_duration: u32,
    pub network_interval_number: u32,
}

impl Args {
    pub fn par() -> Self {
        let args = Self::parse();
        args
    }
    pub fn network_config(&self) -> NetworkConfig {
        let path = {
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
        };

        let disable_network_statistics = if self.disable_network_statistics {
            true
        } else if !self.disable_network_statistics && path.is_empty() {
            false
        } else {
            false
        };

        NetworkConfig {
            disable_network_statistics,
            network_interval: self.network_interval,
            network_save_path: path,
            traffic_period: self.traffic_period.clone(),
            traffic_reset_day: self.traffic_reset_day.clone(),
            network_statistics_mode: self.network_statistics_mode.clone(),
            network_duration: self.network_duration,
            network_interval_number: self.network_interval_number,
        }
    }
}

impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Configuration:")?;

        if let Some(http_server) = &self.http_server {
            writeln!(f, "  HTTP Server: {}", http_server)?;
        }

        if let Some(ws_server) = &self.ws_server {
            writeln!(f, "  WebSocket Server: {}", ws_server)?;
        }

        if let Some(token) = &self.token {
            writeln!(f, "  Token: {}", token)?;
        }

        if self.fake != 1.0 {
            writeln!(f, "  Fake Multiplier: {}", self.fake)?;
        }

        if self.tls {
            writeln!(f, "  TLS Enabled: true")?;
        }

        if self.ignore_unsafe_cert {
            writeln!(f, "  Ignore Unsafe Certificates: true")?;
        }

        if self.dry_run {
            writeln!(f, "  Dry Run Mode: enabled")?;
        }

        writeln!(f, "  Log Level: {:?}", self.log_level)?;
        writeln!(f, "  IP Provider: {:?}", self.ip_provider)?;

        writeln!(
            f,
            "  Real-time Info Interval: {} ms",
            self.realtime_info_interval
        )?;

        writeln!(
            f,
            "  Disable Windows Toast Notify: {}",
            self.disable_toast_notify
        )?;

        writeln!(
            f,
            "  Network Statistics: {}",
            if self.disable_network_statistics {
                "disabled"
            } else {
                "enabled"
            }
        )?;

        if !self.disable_network_statistics {
            writeln!(f, "    Reset Period: {:?}", self.traffic_period)?;
            writeln!(f, "    Reset Day: {}", self.traffic_reset_day)?;
            writeln!(f, "    Save Interval: {} s", self.network_interval)?;
            if let Some(save_path) = &self.network_save_path {
                writeln!(f, "    Save Path: {}", save_path)?;
            } else {
                writeln!(f, "    Save Path: auto-determined")?;
            }
        }

        Ok(())
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

#[derive(Serialize, Deserialize, ValueEnum, Debug, Clone, PartialEq)]
pub enum TrafficPeriod {
    Week,
    Month,
    Year,
}

#[derive(Serialize, Deserialize, ValueEnum, Debug, Clone, PartialEq)]
pub enum NetworkStatisticsMode {
    Natural,
    Fixed,
}
fn network_statistics_mode() -> NetworkStatisticsMode {
    NetworkStatisticsMode::Fixed
}

fn traffic_period() -> TrafficPeriod {
    TrafficPeriod::Month
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
