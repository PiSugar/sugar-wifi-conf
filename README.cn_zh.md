# sugar-wifi-conf

<p>
  <img width="200" src="https://github.com/user-attachments/assets/e620a5b4-a788-4b72-8f49-fe186f2bf7fa" />
  <img width="200" src="https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg" />
</p>

[English](https://github.com/PiSugar/sugar-wifi-conf) | 简体中文 

让树莓派提供蓝牙BLE服务，使用PiSugar APP/小程序即可随时更改树莓派的wifi连接，获取wifi名称和ip地址等信息，也可以自定义显示系统信息，接受和执行shell命令。

适用于带有蓝牙的树莓派型号(已测试5B/4B/3B+/zero w/zero 2w)，在Raspbain官方镜像可运行。

PiSugar APP / 微信小程序 请扫描上方二维码获取。

小程序源代码详见sugar-wifi-miniapp文件夹。

你也可以使用web蓝牙来连接，请确保你的设备和浏览器支持web-bluetooth api. (已在MacOS和Android的chrome浏览器上测试，iOS可使用[WebBLE](https://apps.apple.com/us/app/webble/id1193531073)浏览器) 使用chrome打开[web蓝牙页面](https://www.pisugar.com/sugar-wifi-conf)进行连接。源码详见web-bluetooth-client文件夹。


### 安装

#### 一键安装（推荐）

下载预编译二进制文件并安装为 systemd 服务，无需 Rust 工具链：

```bash
curl -sSL https://repo.pisugar.uk/PiSugar/sugar-wifi-conf/raw/master/install-bin.sh | sudo bash
```

安装指定版本：
```bash
curl -sSL https://repo.pisugar.uk/PiSugar/sugar-wifi-conf/raw/master/install-bin.sh | sudo bash -s -- v0.1.0
```

**支持的树莓派型号：**

| 型号 | 架构 | 二进制文件 |
| --- | --- | --- |
| Pi Zero, Zero W, Pi 1 | ARMv6 (armv6l) | `sugar-wifi-conf-armv6` |
| Pi 2, 3, 4, Zero 2 W (32位系统) | ARMv7 (armv7l) | `sugar-wifi-conf-armv7` |
| Pi 3, 4, 5, Zero 2 W (64位系统) | AArch64 (aarch64) | `sugar-wifi-conf-aarch64` |

安装脚本会自动检测架构并下载对应的二进制文件。

#### 从源码编译

源码在 `/rust` 目录。

```
# 在树莓派上编译安装
cd rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
cargo build --release
sudo mkdir -p /opt/sugar-wifi-config
sudo cp target/release/sugar-wifi-conf /opt/sugar-wifi-config/
sudo cp ../custom_config.json /opt/sugar-wifi-config/
```

或使用安装脚本：
```
cd rust && sudo bash install.sh
```

### 可选参数
```
# 可修改 /etc/systemd/system/sugar-wifi-config.service 文件改变运行参数
# 例如：
sugar-wifi-conf serve --name raspberrypi --key mykey --config /opt/sugar-wifi-config/custom_config.json
```

安装完成后重启树莓派。进入PiSugar APP的Wifi Config页面或者使用微信扫描二维码进入小程序，即可控制树莓派。

### 通过 BLE 进行 SSH 连接

Rust 版本内置了 SSH 隧道功能，可以通过蓝牙 BLE 直接 SSH 到树莓派，无需 WiFi 或 IP 地址。macOS 命令行客户端在 `/ble-ssh-client` 目录。

#### 编译客户端（macOS）

```bash
cd ble-ssh-client
cargo build --release
# 二进制文件在 target/release/ble-ssh
```

#### 使用方法

```bash
# 自动模式：扫描 BLE 设备，交互式选择设备，IP 可达时优先使用 IP
ble-ssh

# 指定 IP 地址（BLE 作为备选）
ble-ssh --ip 192.168.1.100

# 纯 IP 模式（不扫描 BLE）
ble-ssh --ip 192.168.1.100 --no-ble

# 强制使用 BLE 隧道（即使 IP 可达）
ble-ssh --force-ble

# 自定义本地端口和扫描超时
ble-ssh --port 2222 --scan-timeout 15
```

发现多个设备时会显示交互式选择列表：
```
  📱 Found: pisugar-a
  📱 Found: pisugar-b

? Select device
> pisugar-a (IP: 192.168.1.100)
  pisugar-b (BLE only)

? Connection mode
> Auto (prefer IP, BLE fallback)
  Force BLE
```

然后在另一个终端连接：
```bash
ssh pi@localhost -p 2222
```

隧道活跃时客户端会显示实时传输速度。

#### 工作原理

1. 客户端扫描 PiSugar BLE 设备并连接。
2. 每次 SSH 连接时，客户端通过 SSH_CTRL 发送 `CONNECT`，服务端连接本地 sshd（127.0.0.1:22）并回复 `OK`。
3. SSH 数据双向传输：客户端 → SSH_RX → sshd，sshd → SSH_TX → 客户端。
4. SSH 会话结束时发送 `DISCONNECT`，服务端回复 `CLOSED`。
5. 如果 Pi 的 IP 可达，客户端会直接通过 TCP 桥接以获得更好速度；否则走 BLE 隧道。

<p align="center">
  <img width="190" src="https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/demo.gif">
  <img width="670" src="https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/miniapp-demo-cn-fix2.jpg">
</p>

### 自定义配置

通过修改配置文件可以让你的树莓派通过BLE更方便的发送系统信息，接受和执行shell命令

注意：若配置文件格式有误或着因权限问题无法读取，小程序端将无法获取自定义的信息。

info为客户端显示的参数，注意command获得的结果不能超过20个字符，interval为每次获取结果的间隔秒数。

commands为客户端可向树莓派发出的shell命令。

```
{
  "info": [
    {
      "label": "CPU Temp",
      "command": "vcgencmd measure_temp | cut -d = -f 2 | awk '{printf \"%s \", $1}'",
      "interval": 5
    },
    {
      "label": "CPU Load",
      "command": "top -bn1 | grep load | awk '{printf \"%.2f%%\", $(NF-2)}'",
      "interval": 1
    },
    {
      "label": "Memory",
      "command": "free -m | awk 'NR==2{printf \"%s/%sMB\", $3,$2 }'",
      "interval": 5
    },
    {
      "label": "UP Time",
      "command": "uptime -p | cut -d 'p' -f 2 | awk '{ printf \"%s\", $0 }'",
      "interval": 10
    }
  ],
  "commands": [
    {
      "label": "shutdown",
      "command": "shutdown"
    },
    {
      "label": "reboot",
      "command": "reboot"
    }
  ]
}

```

### 蓝牙BLE服务详细参数

服务uuid: FD2B-4448-AA0F-4A15-A62F-EB0BE77A0000

| 特征值 | uuid | 属性 | 说明 |
| - | :- | :- | :- |
| SERVICE_NAME | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0001 | read | 服务名称，固定值 |
| DEVICE_MODEL | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0002 | read | 树莓派版本 |
| WIFI_NAME | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0003 | notify | 正在连接的wifi名称 |
| IP_ADDRESS | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0004 | notify | 现有内网ip地址 |
| INPUT | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0005 | write | 输入wifi配置信息（已弃用） |
| NOTIFY_MESSAGE | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0006 | notify | wifi配置操作返回的信息 |
| INPUT_SEP | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0007 | write | 输入wifi配置信息（分包） |
| CUSTOM_COMMAND_INPUT | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0008 | write | 输入自定义命令（分包） |
| CUSTOM_COMMAND_NOTIFY | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0009 | notify | 命令执行返回（分包） |
| CUSTOM_INFO_LABEL | 0000-0000-0000-0000-0000-FD2BCCCAXXXX | read | 自定义信息显示的标签名 |
| CUSTOM_INFO | 0000-0000-0000-0000-0000-FD2BCCCBXXXX | notify | 自定义信息显示的数值 |
| CUSTOM_COMMAND_LABEL | 0000-0000-0000-0000-0000-FD2BCCCCXXXX | read | 自定义命令名称 |
| SSH_CTRL | FD2B-4448-AA0F-4A15-A62F-EB0BE77A000A | write, notify | SSH 隧道控制（CONNECT / DISCONNECT） |
| SSH_RX | FD2B-4448-AA0F-4A15-A62F-EB0BE77A000B | write | SSH 数据：客户端 → Pi（原始二进制，512 字节分包） |
| SSH_TX | FD2B-4448-AA0F-4A15-A62F-EB0BE77A000C | notify | SSH 数据：Pi → 客户端（原始二进制，512 字节分包） |


### 通讯格式

| 特征值 | 操作说明 |
| - | :- |
| INPUT_SEP | 发送格式为 key%&%ssid%&%password&#& （分为多条20字节数据传送），例如：pisugar%&%home_wifi%&%12345678&#& |
| CUSTOM_COMMAND_INPUT | 发送格式为 key%&%4_digit_uuid&#&（分为多条20字节数据传送，例如：key%&%1234&#& 将执行CUSTOM_COMMAND_LABEL末四位uuid为1234的命令） |
| CUSTOM_COMMAND_NOTIFY | 分为多条20字节数据传送，结束符为&#& |
| CUSTOM_INFO_LABEL | 例如：uuid为FD2BCCCA1234的CUSTOM_INFO_LABEL 对应 uuid为FD2BCCCB1234的CUSTOM_INFO特征值 |
| CUSTOM_COMMAND_LABEL | 所有的自定义命令都会广播成uuid为 FD2BCCCCXXXX 的特征值，读取特征值可以获取命令的label, uuid后四位可以用发送执行 |
| SSH_CTRL | 写入 `CONNECT` 打开到 sshd 的隧道，服务端通知 `OK` 或 `ERR:<原因>`。写入 `DISCONNECT` 关闭隧道，服务端通知 `CLOSED`。 |
| SSH_RX | 原始二进制 SSH 数据，客户端写入后转发到 sshd。每次写入最大 512 字节。 |
| SSH_TX | 原始二进制 SSH 数据，sshd 发送到客户端的 BLE 通知。每次通知最大 512 字节。 |


