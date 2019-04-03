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

# 第二个参数是自定义配置json文件地址，如需显示cpu，内存等自定义信息
# 请参照custom_display.json文件创建配置文件，并将文件路径作为第二个参数传入，例如：
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf pisugar /home/pi/sugar-wifi-conf/custom_display.json

```






