use log::{error, info};
use miniserde::{Deserialize, Serialize};
use palc::{Parser, ValueEnum};
use std::path::PathBuf;
use std::process::exit;
use std::{env, fs};

#[derive(Parser, Debug, Clone)]
#[command(
    version,
    long_about = "komari-monitor-rs is a third-party high-performance monitoring agent for the komari monitoring service.",
    after_long_help = "必须设置 --http-server / --token\n--ip-provider 接受 cloudflare / ipinfo\n--log-level 接受 error, warn, info, debug, trace\n\n本 Agent 开源于 Github , 使用强力的 Rust 驱动, 爱来自 Komari"
)]
pub struct Args {
    // Main
    /// 设置主端 Http 地址
    #[arg(long)]
    pub http_server: String,

    /// 设置主端 WebSocket 地址
    #[arg(long)]
    pub ws_server: Option<String>,

    /// 设置 Token
    #[arg(short, long, allow_hyphen_values = true)]
    pub token: String,

    /// 设置虚假倍率
    #[arg(short, long, default_value_t = 1.0)]
    pub fake: f64,

    /// 启用 TLS (默认关闭)
    #[arg(long, default_value_t = false)]
    pub tls: bool,

    /// 忽略证书验证
    #[arg(long, default_value_t = false)]
    pub ignore_unsafe_cert: bool,

    /// 设置日志等级 (反馈问题请开启 Debug 或者 Trace)
    #[arg(long, default_value_t = log_level())]
    pub log_level: LogLevel,

    // Other
    /// 公网 IP 接口
    #[arg(long, default_value_t=ip_provider())]
    pub ip_provider: IpProvider,

    /// 启用 Terminal (默认关闭)
    #[arg(long, default_value_t = false)]
    pub terminal: bool,

    /// 自定义 Terminal 入口
    #[arg(long, default_value_t = terminal_entry())]
    pub terminal_entry: String,

    /// 设置 Real-Time Info 上传间隔时间 (ms)
    #[arg(long, default_value_t = 1000)]
    pub realtime_info_interval: u64,

    // Network
    /// 关闭网络流量统计
    #[arg(long, default_value_t = false)]
    pub disable_network_statistics: bool,

    /// 网络流量统计保存时长 (s)
    #[arg(long, default_value_t = 864000)]
    pub network_duration: u32,

    /// 网络流量统计间隔 (s)
    #[arg(long, default_value_t = 10)]
    pub network_interval: u32,

    /// 网络流量统计保存到磁盘间隔次数 (s)
    #[arg(long, default_value_t = 10)]
    pub network_interval_number: u32,

    /// 网络统计保存地址
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
        let mut args = Self::parse();
        unsafe {
            crate::get_info::network::DURATION = args.realtime_info_interval as f64;
        }
        if args.terminal_entry == "default" {
            args.terminal_entry = {
                if cfg!(windows) {
                    "cmd.exe".to_string()
                } else if fs::exists("/bin/bash").unwrap_or(false) {
                    "bash".to_string()
                } else {
                    "sh".to_string()
                }
            };
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
                            let home = match env::var("HOME") {
                                Ok(home) => home,
                                Err(e) => {
                                    error!(
                                        "无法自动获取 网络流量信息 文件保存地址，请手动指定: {}",
                                        e
                                    );
                                    exit(1);
                                }
                            };

                            PathBuf::from(home)
                                .join(".config/komari-network.conf")
                                .to_string_lossy()
                                .to_string()
                        };
                        info!("已自动获取 网络流量信息 文件保存地址: {}", path);
                        path
                    }
                } else {
                    let path = PathBuf::from(self.network_save_path.as_ref().unwrap())
                        .to_string_lossy()
                        .to_string();
                    info!("已获取 网络流量信息 文件保存地址: {}", path);
                    path
                }
            },
        }
    }
}

// Default Settings

fn terminal_entry() -> String {
    "default".to_string()
}

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
