#!/bin/bash
NVM_URL="https://cdn.pisugar.com/PiSugar-wificonfig/script/install_nvm.sh"
REPO_URL="https://gitee.com/jdaie/sugar-wifi-config.git"
INSTALL_DIR="/opt/sugar-wifi-config"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install nvm and Node.js 18
install_node() {
    echo "Installing Node.js 18 using nvm..."
    
    # Install nvm if it's not already installed
    if [ ! -d "$HOME/.nvm" ]; then
        echo "Installing nvm..."
        curl -o- $NVM_URL | bash
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
    else
        echo "nvm is already installed."
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        [ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
    fi

    # Install and use Node.js 18
    nvm install 18
    nvm use 18

    # Verify installation
    if command_exists node && [[ "$(node -v)" =~ ^v18 ]]; then
        echo "Node.js 18 installed successfully."
    else
        echo "Failed to install Node.js 18."
        exit 1
    fi
}

# Check if Node.js is installed and is version 18
if command_exists node; then
    NODE_VERSION=$(node -v)
    if [[ "$NODE_VERSION" =~ ^v18 ]]; then
        echo "Node.js 18 is already installed."
    else
        echo "Different version of Node.js detected: $NODE_VERSION"
        install_node
    fi
else
    echo "Node.js is not installed."
    install_node
fi

# Check if npm is installed
if ! command_exists npm; then
    echo "npm is not installed. Installing npm..."
    sudo apt-get install -y npm

    # Verify installation
    if command_exists npm; then
        echo "npm installed successfully."
    else
        echo "Failed to install npm."
        exit 1
    fi
fi

#sudo ln -s "$NVM_DIR/versions/node/$(nvm version)/bin/node" "/usr/local/bin/node"
#sudo ln -s "$NVM_DIR/versions/node/$(nvm version)/bin/npm" "/usr/local/bin/npm"
#sudo ln -s "$NVM_DIR/versions/node/$(nvm version)/bin/npx" "/usr/local/bin/npx"

# install repo
# Check if /opt/sugar-wifi-config exists and delete it if it does
if [ -d "$INSTALL_DIR" ]; then
    echo "$INSTALL_DIR exists. Deleting..."
    sudo rm -rf "$INSTALL_DIR"
    if [ ! -d "$INSTALL_DIR" ]; then
        echo "$INSTALL_DIR successfully deleted."
    else
        echo "Failed to delete $INSTALL_DIR."
        exit 1
    fi
else
    echo "$INSTALL_DIR does not exist."
fi

echo "Cloning $REPO_URL to $INSTALL_DIR..."
mkdir -p $INSTALL_DIR
git clone $REPO_URL $INSTALL_DIR
cd $INSTALL_DIR
git pull

echo "Installing dependencies..."
npm i

chmod +x $INSTALL_DIR/run.sh

# /etc/rc.local
echo "Adding startup command to /etc/rc.local..."
mkdir -p $INSTALL_DIR/build
cp custom_config.json $INSTALL_DIR/build/custom_config.json

# Check if /etc/rc.local exists and add the startup command if it does
# sudo sed -i "/exit 0/i sudo bash $INSTALL_DIR/run.sh pisugar $INSTALL_DIR/build/custom_config.json&" /etc/rc.local
echo -e "Add to startup..."
sudo sed -i '/sugar-wifi-conf/d' /etc/rc.local
sudo sed -i 's/"exit 0"/"Cross the wall, we can reach every corner of the world"/' /etc/rc.local
sudo sed -i '/exit 0/i sudo bash /opt/sugar-wifi-config/run.sh pisugar /opt/sugar-wifi-config/build/custom_config.json&' /etc/rc.local
sudo sed -i 's/"Cross the wall, we can reach every corner of the world"/"exit 0"/' /etc/rc.local
echo -e "Well done Pi Star people!"
echo -e "Please restart your raspberry pi and enjoy it!!"

