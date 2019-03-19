# sugar-wifi-conf

![PiSugar MiniAPP](https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/image/qrcode.jpg)

让树莓派提供蓝牙BLE服务，使用小程序即可随时更改树莓派的wifi连接，获取wifi名称和ip地址等信息。


### 安装步骤

安装 NetworkManager

```
sudo apt-get update
sudo apt-get install network-manager

# 在NetworkManager.conf中，将managed=false改为managed=true
sudo nano /etc/NetworkManager/NetworkManager.conf

# 在dhcpcd.conf文件末尾加上一行：denyinterfaces wlan0
sudo nano /etc/dhcpcd.conf

# 确保wlan0没有被interface文件引用，被引用的话可能会造成 NetworkManager 无法接管wifi
sudo nano /etc/network/interfaces

```

下载项目文件，示例是下载在pi用户目录下

```
cd ~
git clone https://github.com/PiSugar/sugar-wifi-conf.git
cd sugar-wifi-conf

```


切换到root下安装 nodejs （蓝牙服务需要root权限）


```
# 切换到root下
sudo su

# 使用nvm安装nodejs 8.0 (blueno依赖暂不支持更高版本)
wget -qO- https://raw.githubusercontent.com/creationix/nvm/v0.31.1/install.sh | bash
source ~/.bashrc
nvm install 8

# 查看是否安装成功，版本为8
node -v

```

在sugar-wifi-conf目录下安装依赖，设置项目开机启动

```
# 安装依赖
npm i

# 安装pm2设置开机启动
npm i pm2 -g
pm2 start index.js
pm2 startup
pm2 save
```

重启后即完成安装，手机开启蓝牙，使用微信小程序即可发现树莓派，进行wifi设置。


注意：NetworkManager接管wifi后，原有的wifi配置会失效。如需自行连接wifi，可使用命令：

```
nmcli device wifi con "wifi名称" password "wifi密码" 

``` 



