ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH=arm64

# Install dev libraries for the target architecture.
# cross-rs containers are Ubuntu-based (amd64) and only have archive.ubuntu.com sources.
# ARM/ARM64 packages live on ports.ubuntu.com — we must rewrite apt sources completely.
RUN set -eux; \
    dpkg --add-architecture ${CROSS_DEB_ARCH}; \
    . /etc/os-release; \
    echo ">>> Distro: ${ID} ${VERSION_CODENAME:-unknown}, target arch: ${CROSS_DEB_ARCH}"; \
    rm -f /etc/apt/sources.list.d/*.sources 2>/dev/null || true; \
    printf '%s\n' \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME} main restricted universe multiverse" \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-updates main restricted universe multiverse" \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-security main restricted universe multiverse" \
        > /etc/apt/sources.list; \
    printf '%s\n' \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME} main universe" \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME}-updates main universe" \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME}-security main universe" \
        > /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libdbus-1-dev:${CROSS_DEB_ARCH} \
        libgtk-3-dev:${CROSS_DEB_ARCH} \
        libxdo-dev:${CROSS_DEB_ARCH}
