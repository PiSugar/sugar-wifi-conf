#/bin/bash
cd /home/pi
echo -e "Download service package"
wget http://cdn.pisugar.com/release/sugar-wifi-conf.tar.gz
echo -e "Unzip package"
tar -xzvf sugar-wifi-conf.tar.gz
rm sugar-wifi-conf.tar.gz
echo -e "Begin install service"
sudo -s . ./sugar-wifi-conf/wificonfig.sh
