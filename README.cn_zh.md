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
```
curl https://cdn.pisugar.com/PiSugar-wificonfig/script/install.sh | sudo bash

# the script will add sugar-wifi-conf to /etc/rc.local so that it can run on startup
```

### 可选参数
```
# 可修改 /etc/systemd/system/sugar-wifi-config.service 文件改变运行参数
# 第一个参数为key
# 第二个参数是自定义配置json文件地址
# 例如：
sudo bash /opt/sugar-wifi-config/run.sh pisugar /opt/sugar-wifi-config/build/custom_config.json
```

安装完成后重启树莓派。进入PiSugar APP的Wifi Config页面或者使用微信扫描二维码进入小程序，即可控制树莓派。

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


### 通讯格式

| 特征值 | 操作说明 |
| - | :- |
| INPUT_SEP | 发送格式为 key%&%ssid%&%password&#& （分为多条20字节数据传送），例如：pisugar%&%home_wifi%&%12345678&#& |
| CUSTOM_COMMAND_INPUT | 发送格式为 key%&%4_digit_uuid&#&（分为多条20字节数据传送，例如：key%&%1234&#& 将执行CUSTOM_COMMAND_LABEL末四位uuid为1234的命令） |
| CUSTOM_COMMAND_NOTIFY | 分为多条20字节数据传送，结束符为&#& |
| CUSTOM_INFO_LABEL | 例如：uuid为FD2BCCCA1234的CUSTOM_INFO_LABEL 对应 uuid为FD2BCCCB1234的CUSTOM_INFO特征值 |
| CUSTOM_COMMAND_LABEL | 所有的自定义命令都会广播成uuid为 FD2BCCCCXXXX 的特征值，读取特征值可以获取命令的label, uuid后四位可以用发送执行 |


