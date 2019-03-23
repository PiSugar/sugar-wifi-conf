#/bin/bash
cd sugar-wifi-conf/build
sudo chmod 777 binding.node
sudo chmod 777 sugar-wifi-conf
sudo sed -i '/sugar-wifi-conf/d' /etc/rc.local
sudo sed -i '/exit 0/i sudo /home/pi/sugar-wifi-conf/build/sugar-wifi-conf' /etc/rc.local
