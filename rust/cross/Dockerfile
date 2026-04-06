ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH

RUN set -eux; \
    dpkg --add-architecture ${CROSS_DEB_ARCH}; \
    # Add multiarch package sources (Ubuntu uses ports.ubuntu.com for foreign architectures)
    if grep -qi ubuntu /etc/os-release; then \
        codename=$(grep VERSION_CODENAME /etc/os-release | cut -d= -f2); \
        echo "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports $codename main universe" \
            > /etc/apt/sources.list.d/cross-arch.list; \
        echo "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports $codename-updates main universe" \
            >> /etc/apt/sources.list.d/cross-arch.list; \
    fi; \
    apt-get update; \
    apt-get install -y --no-install-recommends libdbus-1-dev:${CROSS_DEB_ARCH} pkg-config
