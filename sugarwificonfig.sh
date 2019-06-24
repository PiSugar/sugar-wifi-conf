#/bin/bash
cd /home/pi
echo -e "Download service package"
echo -e "Unzip package"
tar -xzvf sugar-wifi-conf.tar.gz
echo -e "Begin install service"
sudo -s . ./sugar-wifi-conf/wificonfig.sh
