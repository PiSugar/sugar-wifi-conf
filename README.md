# sugar-wifi-conf

![PiSugar MiniAPP](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)

让树莓派提供蓝牙BLE服务，使用小程序即可随时更改树莓派的wifi连接，获取wifi名称和ip地址等信息。


### 安装步骤

下载项目文件，示例是下载在pi用户目录下

```
cd ~
git clone https://github.com/PiSugar/sugar-wifi-conf.git
cd sugar-wifi-conf/build

# 修改文件权限
chmod 777 binding.node
chmod 777 sugar-wifi-conf

# 测试是否可以运行
sudo ./sugar-wifi-conf

# 设置开机启动
sudo nano /etc/rc.local
# 在exit0之前添加一行： sudo ./home/pi/sugar-wifi-conf/build/sugar-wifi-conf
# 重启后即可使用！

# 若想改变蓝牙设置的key，可在执行命令后面加一个参数，比如讲key改为123456，可以这样设置：
sudo ./home/pi/sugar-wifi-conf/build/sugar-wifi-conf 123456

```





