# sugar-wifi-conf

![PiSugar MiniAPP](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)


A BLE service to configure wifi over bluetooth for a Raspberry Pi. You can:

- get wifi name, ip address, pi model
- config wifi
- get other custom info
- remote control the pi to execute shell script and get response 

Tested on Raspberry Pi 3B/3B+/zero w (models with bluetooth), Raspbian.


### Setup
```
git clone https://github.com/PiSugar/sugar-wifi-conf.git
sudo -s . ./sugar-wifi-conf/wificonfig.sh

## optional parameters

# edit /etc/rc.local to append parameters to execute path 
# param 1: custom key 
# param 2: path to custom config file
# example: 
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf pisugar /home/pi/sugar-wifi-conf/custom_config.json

```

![PiSugar MiniAPP Demo](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/demo.gif)


### By editing the custom config file, you can let the pi broadcast custom data, recieve and execute preset shell scripts

note: please ensure that the config file is accessable. There are two different types of custom item. 


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


### Input schemes

| charateristic | manual |
| - | :- |
| INPUT_SEP | scheme: key&%&ssid&%&password%#% (subcontract in 20 btyes) example：pisugar&%&home_wifi&%&12345678%#% |
| CUSTOM_COMMAND_INPUT | scheme: key&%&custom_command_label_uuid%#% (subcontract in 20 btyes, custom_command_label_uuid) |
| CUSTOM_COMMAND_NOTIFY | subcontract in 20 btyes, ended in "%#%" |
| CUSTOM_INFO_LABEL | a custom info label (FD2BCCCA1234) will have a corresponding value (FD2BCCCB1234) |



