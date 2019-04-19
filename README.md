# sugar-wifi-conf

![PiSugar Wechat MiniApp](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)

English | [简体中文](https://github.com/PiSugar/sugar-wifi-conf/blob/master/README.cn_zh.md)

A BLE service to configure wifi over bluetooth for a Raspberry Pi. You can:

- get wifi name, ip address, pi model
- config wifi
- get other custom info, e.g. CPU tempreture, CPU load
- remote control the pi to execute shell script and get response, such as shutdown, reboot 

Tested on Raspberry Pi 3B/3B+/zero w (models with bluetooth), Raspbian.

This project is a server-side program. To access client-side app, please use Wechat app to scan the QR-code above.

<p align="center">
  <img width="190" src="https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/demo.gif">
  <img width="670" src="https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/miniapp-demo-en-fix2.jpg">
</p>

### Setup
```
git clone https://github.com/PiSugar/sugar-wifi-conf.git
sudo -s . ./sugar-wifi-conf/wificonfig.sh

# the scrpit will add sugar-wifi-conf to /etc/rc.local so that it can run on startup

## optional parameters

# edit /etc/rc.local to append parameters to execute path 
# param 1: key 
# param 2: path to custom config file
# example: 
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf pisugar /home/pi/sugar-wifi-conf/custom_config.json

```

### Custom configuration

By editing the custom config file, you can let the pi broadcast custom data, recieve and execute custom shell scripts. Note: please ensure that the config file is accessable for the program.

custom_config.json example

```
{
  "note": {
    "info" : {
      "label": "name of the item, within 20 bytes",
      "command": "the command to get value of the item, within 20 bytes",
      "interval": "run command to get data in every X seconds"
    },
   "commands": {
      "label": "name of the item, within 20 bytes",
      "command": "the command to execute"
    }
  },
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
      "label": "ls",
      "command": "ls"
    },
    {
      "label": "shutdown",
      "command": "shutdown"
    },
    {
      "label": "cancel shutdown",
      "command": "shutdown -c"
    },
    {
      "label": "reboot",
      "command": "reboot"
    }
  ]
}

```

### BLE datasheet

You can build your own client-side app base on this datasheet.

Service uuid: FD2B-4448-AA0F-4A15-A62F-EB0BE77A0000

| charateristic | uuid | properties | note |
| - | :- | :- | :- |
| SERVICE_NAME | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0001 | read | service name |
| DEVICE_MODEL | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0002 | read | pi model info |
| WIFI_NAME | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0003 | notify | current wifi name |
| IP_ADDRESS | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0004 | notify | internal ip addresses |
| INPUT | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0005 | write | input for configuring wifi (deprecated) |
| NOTIFY_MESSAGE | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0006 | notify | response for configuring wifi |
| INPUT_SEP | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0007 | write | input for configuring wifi（subcontracting） |
| CUSTOM_COMMAND_INPUT | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0008 | write | input for custom commands（subcontracting） |
| CUSTOM_COMMAND_NOTIFY | FD2B-4448-AA0F-4A15-A62F-EB0BE77A0009 | notify | response for custom commands（subcontracting） |
| CUSTOM_INFO_LABEL | 0000-0000-0000-0000-0000-FD2BCCCAXXXX | read | label of custom info |
| CUSTOM_INFO | 0000-0000-0000-0000-0000-FD2BCCCBXXXX | notify | value of custom info |
| CUSTOM_COMMAND_LABEL | 0000-0000-0000-0000-0000-FD2BCCCCXXXX | read | label of custom command |


### Input and Output format

| charateristic | format |
| - | :- |
| INPUT_SEP | format: key&%&ssid&%&password%#% (subcontract in 20 btyes) e.g. pisugar&%&home_wifi&%&12345678%#% |
| CUSTOM_COMMAND_INPUT | format: key&%&last_4_digit_uuid%#% (subcontract in 20 btyes) e.g. pisugar&%&1234%#% will execute the custom command with its label uuid end in "1234" |
| CUSTOM_COMMAND_NOTIFY | subcontract in 20 btyes, ended in "%#%" |
| CUSTOM_INFO_LABEL | a custom info label (FD2BCCCA1234) will have a corresponding value (FD2BCCCB1234) |
| CUSTOM_COMMAND_LABEL | all custom commands with be broadcast in uuid "FD2BCCCCXXXX" |



