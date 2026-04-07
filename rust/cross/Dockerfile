ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH=armhf

# Install libdbus-1-dev for the target architecture.
# cross-rs containers are Ubuntu-based (amd64) and only have archive.ubuntu.com sources.
# ARM/ARM64 packages live on ports.ubuntu.com — we must rewrite apt sources completely
# to avoid conflicts between archive.ubuntu.com (no armhf) and ports.ubuntu.com.
RUN set -eux; \
    dpkg --add-architecture ${CROSS_DEB_ARCH}; \
    . /etc/os-release; \
    echo ">>> Distro: ${ID} ${VERSION_CODENAME:-unknown}, target arch: ${CROSS_DEB_ARCH}"; \
    # Remove DEB822 format sources (Ubuntu 24.04+ uses .sources files)
    rm -f /etc/apt/sources.list.d/*.sources 2>/dev/null || true; \
    # Overwrite sources.list — pin amd64 to archive.ubuntu.com
    printf '%s\n' \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME} main restricted universe multiverse" \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-updates main restricted universe multiverse" \
        "deb [arch=amd64] http://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-security main restricted universe multiverse" \
        > /etc/apt/sources.list; \
    # Add foreign arch from ports.ubuntu.com
    printf '%s\n' \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME} main universe" \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME}-updates main universe" \
        "deb [arch=${CROSS_DEB_ARCH}] http://ports.ubuntu.com/ubuntu-ports ${VERSION_CODENAME}-security main universe" \
        > /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
    echo '=== sources.list ==='; cat /etc/apt/sources.list; \
    echo "=== cross-${CROSS_DEB_ARCH}.list ==="; cat /etc/apt/sources.list.d/cross-${CROSS_DEB_ARCH}.list; \
    apt-get update; \
    apt-get install -y --no-install-recommends libdbus-1-dev:${CROSS_DEB_ARCH} pkg-config
