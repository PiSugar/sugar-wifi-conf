# sugar-wifi-conf

![PiSugar MiniAPP](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)

让树莓派提供蓝牙BLE服务，使用小程序即可随时更改树莓派的wifi连接，获取wifi名称和ip地址等信息。
适用于带有蓝牙的树莓派型号(已测试3B+, zero w)，在Raspbain官方镜像可运行


### 安装步骤


```
#下载项目文件，示例是下载在pi用户目录下
cd ~
git clone https://github.com/PiSugar/sugar-wifi-conf.git
cd sugar-wifi-conf/build

# 修改文件权限
chmod 777 binding.node
chmod 777 sugar-wifi-conf

# 测试是否可以运行，运行后使用微信小程序扫描。
# 注意此时更改wifi可能会造成网络断开，程序结束。所以建议在设置开机启动后再测试修改wifi
sudo sugar-wifi-conf

# 设置开机启动
sudo nano /etc/rc.local
# 在exit 0之前添加一行： sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf
# 重启后即可使用！

# 若想改变蓝牙设置的key，可在执行命令后面加一个参数，如果要将key改为123456，可以这样设置：
sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf 123456

#不设置的话key将默认为pisugar

```





