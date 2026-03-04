FROM rust:1-bookworm

# Install Zig toolchain and the armhf development headers we need for Slint
# (fontconfig, input stack, DRM/GBM, etc.).
RUN dpkg --add-architecture armhf \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        pkg-config \
        gcc-arm-linux-gnueabihf \
        libc6-dev-armhf-cross \
        libfontconfig1-dev:armhf \
        libxkbcommon-dev:armhf \
        libudev-dev:armhf \
        libdrm-dev:armhf \
        libgbm-dev:armhf \
        libinput-dev:armhf \
        libegl1-mesa-dev:armhf \
        libgles2-mesa-dev:armhf \
        zig \
    && rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_LIBDIR=/usr/lib/arm-linux-gnueabihf/pkgconfig:/usr/share/pkgconfig
ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig

WORKDIR /project

# Default command runs cargo-zigbuild for the armv7 target.
CMD ["cargo", "zigbuild", "--target", "armv7-unknown-linux-gnueabihf", "--release"]
