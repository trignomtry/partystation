FROM --platform=linux/amd64 ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:0.2.5

RUN dpkg --add-architecture armhf \
    && apt-get update \
    && apt-get install -y \
        libfontconfig1-dev:armhf \
        libxkbcommon-dev:armhf \
        libudev-dev:armhf \
        libdrm-dev:armhf \
        libgbm-dev:armhf \
        libinput-dev:armhf \
    && rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_LIBDIR=/usr/lib/arm-linux-gnueabihf/pkgconfig:/usr/share/pkgconfig
ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig
