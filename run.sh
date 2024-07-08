#!/bin/bash
NVM_URL="https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.3/install.sh"
REPO_URL="https://github.com/PiSugar/sugar-wifi-conf.git"
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

echo "Starting service..."
cd $INSTALL_DIR
# 运行node index.js 并将命令参数拼接到后面
node index.js $@
