# Komari-Monitor-rs

![](https://hitscounter.dev/api/hit?url=https%3A%2F%2Fgithub.com%2Frsbench%2Frsbench&label=&icon=github&color=%23160d27)
![komari-monitor-rs](https://socialify.git.ci/GenshinMinecraft/komari-monitor-rs/image?custom_description=Komari+%E7%AC%AC%E4%B8%89%E6%96%B9+Agent+%7C+%E9%AB%98%E6%80%A7%E8%83%BD&description=1&font=KoHo&forks=1&issues=1&language=1&name=1&owner=1&pattern=Floating+Cogs&pulls=1&stargazers=1&theme=Auto)

## About

`Komari-Monitor-rs` 是一个适用于 [komari-monitor](https://github.com/komari-monitor) 监控服务的第三方**高性能**监控
Agent

致力于实现[原版 Agent](https://github.com/komari-monitor/komari-agent) 的所有功能，并拓展更多功能

## 近期更新

### Dry Run 支持

现在可以不提供任何参数，仅提供 `--dry-run` 参数，以事先获取监控数据

每次正常运行前也将获取一次数据，若有误监控的项目请发送 DryRun 的输出到 Issue 中，比如各种不应该读取的硬盘、虚拟网卡等

```
The following is the equipment that will be put into operation and monitored:
CPU: AMD EPYC 7763 64-Core Processor, Cores: 4
Memory: 2092 MB / 16773 MB
Swap: 0 MB / 0 MB
Load: 0.36 / 0.65 / 0.37

Hard drives will be monitored:
/dev/root | ext4 | /usr/sbin/docker-init | 8 GB / 31 GB

Network interfaces will be monitored:
eth0 | 00:22:48:58:ca:62 | UP: 0 GB / DOWN: 7 GB
CONNS: TCP: 12 | UDP: 4
```

### 已支持周期流量统计 / 清零

相关参数:

- `--disable-network-statistics`: 禁用周期流量统计，上报的总流量回退到原来自网卡启动以来的总流量，默认关闭
- `--network-duration`: 周期流量统计 的统计长度，单位 sec，默认 864000 (10 Days)
- `--network-interval`: 周期流量统计 的间隔长度，单位 sec，默认 10
- `--network-interval-number`: 周期流量统计 的保存到磁盘间隔次数，默认 10 (该参数意义为 `硬盘读写间隔时间 = 间隔长度 \* 间隔次数`，默认值为 10 * 10 = 100sec 写入一次硬盘)
- `--network-save-path`: 周期流量统计 的文件保存地址，在 Windows 下默认为 `C:\komari-network.conf`，非 Windows 默认为 `/etc/komari-network.conf` (root) 或 `$HOME/.config/komari-network.conf` (非 root)

该功能暂未稳定，有问题请及时反馈

## 一键脚本

**本脚本已不再支持，该项目不面向小白用户，请自行配置**

## 与原版的差异

测试项目均在 Redmi Book Pro 15 2022 锐龙版 + Arch Linux 最新版 + Rust Toolchain Stable 下测试

### Binary 体积

原版体积约 6.2M，本项目体积约 992K，相差约 7.1 倍

### 运行内存与 Cpu 占用

原版占用内存约 15.4 MiB，本项目占用内存约 5.53 MB，相差约 2.7 倍

原版峰值 Cpu 占用约 49.6%，本项目峰值 Cpu 占用约 4.8%

并且，本项目在堆上的内存仅 388 kB

### 实现功能

目前，本项目已经实现原版的大部分功能，但还有以下的差异:

- GPU Name 检测

除此之外，还有希望添加的功能:

- 自动更新
- ~~自动安装~~
- ~~Bash / PWSH 一键脚本~~

## 下载

在本项目的 [Release 界面](https://github.com/GenshinMinecraft/komari-monitor-rs/releases/tag/latest) 即可下载，按照架构选择即可

后缀有 `musl` 字样的可以在任何 Linux 系统下运行

后缀有 `gnu` 字样的仅可以在较新的，通用的，带有 `Glibc` 的 Linux 系统下运行，占用会小一些

## Usage

```
komari-monitor-rs is a third-party high-performance monitoring agent for the komari monitoring service.

Usage: komari-monitor-rs.exe --http-server <HTTP_SERVER> --token <TOKEN> [OPTIONS]

Options:
      --http-server <HTTP_SERVER>
          设置主端 Http 地址

      --ws-server <WS_SERVER>
          设置主端 WebSocket 地址

  -t, --token <TOKEN>
          设置 Token

  -f, --fake <FAKE>
          设置虚假倍率
          [default: 1]

      --tls
          启用 TLS (默认关闭)
          [default: false]

      --ignore-unsafe-cert
          忽略证书验证
          [default: false]

      --log-level <LOG_LEVEL>
          设置日志等级 (反馈问题请开启 Debug 或者 Trace)
          [default: info]

      --ip-provider <IP_PROVIDER>
          公网 IP 接口
          [default: ipinfo]

      --terminal
          启用 Terminal (默认关闭)
          [default: false]

      --terminal-entry <TERMINAL_ENTRY>
          自定义 Terminal 入口
          [default: default]

      --realtime-info-interval <REALTIME_INFO_INTERVAL>
          设置 Real-Time Info 上传间隔时间 (ms)
          [default: 1000]

      --network-statistics
          启用网络流量统计
          [default: true]

      --network-duration <NETWORK_DURATION>
          网络流量统计保存时长 (s)
          [default: 864000]

      --network-interval <NETWORK_INTERVAL>
          网络流量统计间隔 (s)
          [default: 10]

      --network-interval-number <NETWORK_INTERVAL_NUMBER>
          网络流量统计保存到磁盘间隔次数 (s)
          [default: 10]

      --network-save-path <NETWORK_SAVE_PATH>
          网络统计保存地址
```

必须设置 `--http-server` / `--token`
`--ip-provider` 接受 `cloudflare` / `ipinfo`
`--log-level` 接受 `error`, `warn`, `info`, `debug`, `trace`

## Nix 安装

如果你使用 Nix / NixOS，可以直接将本仓库作为 Flake 引入使用：

> [!WARNING]
> 以下是最小化示例配置，单独使用无法工作

```nix
{
  # 将 komari-monitor-rs 作为 flake 引入
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    komari-monitor-rs = {
      url = "github:GenshinMinecraft/komari-monitor-rs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { nixpkgs, komari-monitor-rs, ... }: {
    nixosConfigurations."nixos" = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        komari-monitor-rs.nixosModules.default
        { pkgs, ...}: {
          # 开启并配置 komari-monitor-rs 服务
          services.komari-monitor-rs = {
            enable = true;
            settings = {
              http-server = "https://komari.example.com:12345";
              ws-server = "ws://ws-komari.example.com:54321";
              token = "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
              ip-provider = "ipinfo";
              terminal = true;
              terminal-entry = "default";
              fake = 1;
              realtime-info-interval = 1000;
              tls = true;
              ignore-unsafe-cert = false;
              log-level = "info";
            };
          };
        }
      ];
    };
  };
}
```

## LICENSE

本项目根据 WTFPL 许可证开源

```
        DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE 
                    Version 2, December 2004 

 Copyright (C) 2004 Sam Hocevar <sam@hocevar.net> 

 Everyone is permitted to copy and distribute verbatim or modified 
 copies of this license document, and changing it is allowed as long 
 as the name is changed. 

            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE 
   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION 

  0. You just DO WHAT THE FUCK YOU WANT TO.
```
