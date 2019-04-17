# sugar-wifi-conf

![PiSugar MiniAPP](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)

让树莓派提供蓝牙BLE服务，使用小程序即可随时更改树莓派的wifi连接，获取wifi名称和ip地址等信息。
适用于带有蓝牙的树莓派型号(已测试3B+, zero w)，在Raspbain官方镜像可运行

### 简易安装步骤
```
git clone https://github.com/PiSugar/sugar-wifi-conf.git
sudo -s . ./sugar-wifi-conf/wificonfig.sh

## 可选参数：

# 程序末尾可以加两个运行参数，可修改/etc/rc.local文件改变运行参数。
# 第一个参数为key，如果要将key改为123456，可以这样设置：
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf 123456

# 第二个参数是自定义配置json文件地址，例如显示cpu，内存等自定义信息，可以通过配置文件让蓝牙传输这些信息。
# 请参照custom_config.json文件创建配置文件，并将文件路径作为第二个参数传入，例如：
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf pisugar /home/pi/sugar-wifi-conf/custom_config.json

```

![PiSugar MiniAPP Demo](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/demo.gif)


自定义配置文件参考以下格式：

若配置文件格式有误或着因权限问题无法读取，小程序端将无法获取自定义的信息。
info为小程序显示的参数，注意command获得的结果不超过20个字符，interval为每次获取结果的间隔秒数。
commands为小程序壳向树莓派发出的shell命令。

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



