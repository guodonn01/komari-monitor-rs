use crate::command_parser::NetworkConfig;
use crate::get_info::network::filter_network;
use log::{error, info, warn};
use miniserde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::io::SeekFrom;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use sysinfo::Networks;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, PartialEq)]
struct NetworkInfo {
    config: NetworkConfig,
    source_tx: u64,
    source_rx: u64,
    latest_tx: u64,
    latest_rx: u64,
    counter: u32,
}

impl NetworkInfo {
    pub fn encode(&self) -> String {
        let mut output = String::new();

        macro_rules! append_line {
            ($key:expr, $value:expr) => {
                output.push_str(&format!("{}={}\n", $key, $value));
            };
        }

        append_line!(
            "disable_network_statistics",
            self.config.disable_network_statistics
        );
        append_line!("network_duration", self.config.network_duration);
        append_line!("network_interval", self.config.network_interval);
        append_line!(
            "network_interval_number",
            self.config.network_interval_number
        );
        append_line!("network_save_path", self.config.network_save_path);

        append_line!("source_tx", self.source_tx);
        append_line!("source_rx", self.source_rx);
        append_line!("latest_tx", self.latest_tx);
        append_line!("latest_rx", self.latest_rx);
        append_line!("counter", self.counter);

        output
    }

    /// 解码器：从 String 解析出 NetworkInfo
    pub fn decode(input: &str) -> Result<Self, String> {
        let mut disable_network_statistics = None;
        let mut network_duration = None;
        let mut network_interval = None;
        let mut network_interval_number = None;
        let mut network_save_path = None;
        let mut source_tx = None;
        let mut source_rx = None;
        let mut latest_tx = None;
        let mut latest_rx = None;
        let mut counter = None;

        for (line_num, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line.split_once('=').ok_or_else(|| {
                format!("Line {}: Format error (expected key=value)", line_num + 1)
            })?;

            let key = key.trim();
            let value = value.trim();

            let parse_err = |type_name: &str| {
                format!(
                    "Line {}: Invalid {} for key '{}'",
                    line_num + 1,
                    type_name,
                    key
                )
            };

            match key {
                "disable_network_statistics" => {
                    disable_network_statistics =
                        Some(FromStr::from_str(value).map_err(|_| parse_err("bool"))?)
                }
                "network_duration" => {
                    network_duration = Some(value.parse().map_err(|_| parse_err("u32"))?)
                }
                "network_interval" => {
                    network_interval = Some(value.parse().map_err(|_| parse_err("u32"))?)
                }
                "network_interval_number" => {
                    network_interval_number = Some(value.parse().map_err(|_| parse_err("u32"))?)
                }
                "network_save_path" => network_save_path = Some(value.to_string()),
                "source_tx" => source_tx = Some(value.parse().map_err(|_| parse_err("u64"))?),
                "source_rx" => source_rx = Some(value.parse().map_err(|_| parse_err("u64"))?),
                "latest_tx" => latest_tx = Some(value.parse().map_err(|_| parse_err("u64"))?),
                "latest_rx" => latest_rx = Some(value.parse().map_err(|_| parse_err("u64"))?),
                "counter" => counter = Some(value.parse().map_err(|_| parse_err("u32"))?),
                _ => {}
            }
        }

        // 组装结构体，检查必填字段是否存在
        Ok(NetworkInfo {
            config: NetworkConfig {
                disable_network_statistics: disable_network_statistics
                    .ok_or("Missing field: disable_network_statistics")?,
                network_duration: network_duration.ok_or("Missing field: network_duration")?,
                network_interval: network_interval.ok_or("Missing field: network_interval")?,
                network_interval_number: network_interval_number
                    .ok_or("Missing field: network_interval_number")?,
                network_save_path: network_save_path.ok_or("Missing field: network_save_path")?,
            },
            source_tx: source_tx.ok_or("Missing field: source_tx")?,
            source_rx: source_rx.ok_or("Missing field: source_rx")?,
            latest_tx: latest_tx.ok_or("Missing field: latest_tx")?,
            latest_rx: latest_rx.ok_or("Missing field: latest_rx")?,
            counter: counter.ok_or("Missing field: counter")?,
        })
    }
}

async fn get_or_init_latest_network_info(network_config: &NetworkConfig) -> (File, NetworkInfo) {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&network_config.network_save_path)
        .await
        .unwrap_or_else(|e| {
            error!("无法打开 网络流量信息 文件, 请检查权限: {e}");
            exit(1);
        });

    let mut raw_data = String::new();
    file.read_to_string(&mut raw_data)
        .await
        .unwrap_or_else(|e| {
            error!("无法读取 网络流量信息 文件: {e}");
            exit(1);
        });

    let raw_network_info = if raw_data.is_empty() {
        let network_info = NetworkInfo {
            config: network_config.clone(),
            source_tx: 0,
            source_rx: 0,
            latest_tx: 0,
            latest_rx: 0,
            counter: network_config.network_duration / network_config.network_interval,
        };
        rewrite_network_info_file(&mut file, network_info.encode())
            .await
            .unwrap_or_else(|e| {
                error!("无法写入 网络流量信息 文件: {e}");
                exit(1);
            });
        info!("网络流量信息 文件无数据，可能为第一次运行或更改了保存路径，已新建");
        network_info
    } else {
        let raw_network_info = NetworkInfo::decode(&raw_data).unwrap_or_else(|e| {
            error!("无法解析 网络流量信息 文件: {e}");
            exit(1);
        });

        if &raw_network_info.config != network_config {
            warn!(
                "网络流量信息 文件配置项与原来不符，即将在 3sec 后覆盖原文件并清零统计数据，若需停止请 Ctrl+C"
            );
            tokio::time::sleep(Duration::from_secs(3)).await;
            warn!("开始清理 网络流量信息");
            let network_info = NetworkInfo {
                config: network_config.clone(),
                source_tx: 0,
                source_rx: 0,
                latest_tx: 0,
                latest_rx: 0,
                counter: network_config.network_duration / network_config.network_interval,
            };
            rewrite_network_info_file(&mut file, network_info.encode())
                .await
                .unwrap_or_else(|e| {
                    error!("无法写入 网络流量信息 文件: {e}");
                    exit(1);
                });
            info!("已清理 网络流量信息");
            network_info
        } else {
            raw_network_info
        }
    };

    let new_network_info = NetworkInfo {
        config: raw_network_info.config,
        source_tx: raw_network_info.source_tx + raw_network_info.latest_tx,
        source_rx: raw_network_info.source_rx + raw_network_info.latest_rx,
        latest_tx: 0,
        latest_rx: 0,
        counter: raw_network_info.counter - 1,
    };

    rewrite_network_info_file(&mut file, new_network_info.encode())
        .await
        .unwrap_or_else(|e| {
            error!("无法写入 网络流量信息 文件: {e}");
            exit(1);
        });

    (file, new_network_info)
}

pub async fn network_saver(
    tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    network_config: &NetworkConfig,
) {
    if network_config.disable_network_statistics {
        return;
    }

    loop {
        let (mut file, mut network_info) = get_or_init_latest_network_info(&network_config).await;
        let mut networks = Networks::new_with_refreshed_list();

        // 新增计数器，用来累计内存更新次数
        let mut memory_update_count = 0;

        loop {
            networks.refresh(true);
            let (_, _, total_up, total_down) = filter_network(&networks);

            network_info = NetworkInfo {
                config: network_info.config,
                source_tx: network_info.source_tx,
                source_rx: network_info.source_rx,
                latest_tx: total_up,
                latest_rx: total_down,
                counter: network_info.counter - 1,
            };

            memory_update_count += 1;

            if memory_update_count >= network_config.network_interval_number
                || network_info.counter == 0
            {
                if network_info.counter == 0 {
                    rewrite_network_info_file(&mut file, String::new())
                        .await
                        .unwrap_or_else(|e| {
                            error!("无法写入 网络流量信息 文件: {e}");
                            exit(1);
                        });
                    info!("已完成一个周期的流量统计，已清空数据");
                    break;
                } else {
                    if let Err(e) =
                        rewrite_network_info_file(&mut file, network_info.encode()).await
                    {
                        error!("无法写入 网络流量信息 文件: {e}");
                        continue;
                    }
                    memory_update_count = 0;
                }
            }

            if let Err(e) = tx
                .send((
                    network_info.latest_tx + network_info.source_tx,
                    network_info.latest_rx + network_info.source_rx,
                ))
                .await
            {
                error!("无法发送 流量数据: {e}");
                continue;
            }

            tokio::time::sleep(Duration::from_secs(
                network_info.config.network_interval as u64,
            ))
            .await;
        }
    }
}

async fn rewrite_network_info_file(
    file: &mut File,
    string: String,
) -> Result<(), Box<dyn std::error::Error>> {
    file.set_len(0).await?;
    file.seek(SeekFrom::Start(0)).await?;
    file.write_all(string.as_bytes()).await?;
    Ok(())
}
