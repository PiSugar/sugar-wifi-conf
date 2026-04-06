ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH

# Install libdbus-1-dev for the target architecture.
# cross-rs containers are Ubuntu-based but only have amd64 apt sources.
# For foreign architectures (armhf, arm64), Ubuntu serves packages from ports.ubuntu.com.
RUN set -eux; \
    dpkg --add-architecture ${CROSS_DEB_ARCH}; \
    # Source os-release for ID and VERSION_CODENAME
    . /etc/os-release; \
    echo ">>> Base distro: ${ID} ${VERSION_CODENAME:-unknown}"; \
    case "${ID}" in \
        ubuntu) \
            # Pin existing sources to native arch only
            sed -i '/\[arch=/!s|^deb http|deb [arch=amd64,i386] http|' /etc/apt/sources.list; \
            # Add ports.ubuntu.com for the foreign architecture
            echo "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME} main universe" \
                > /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
            echo "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME}-updates main universe" \
                >> /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
            cat /etc/apt/sources.list; \
            cat /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
            ;; \
        *) \
            # Debian serves all architectures from the same mirror — nothing extra needed
            echo ">>> Non-Ubuntu distro (${ID}), relying on default sources"; \
            ;; \
    esac; \
    apt-get update; \
    apt-get install -y --no-install-recommends libdbus-1-dev:${CROSS_DEB_ARCH} pkg-config
