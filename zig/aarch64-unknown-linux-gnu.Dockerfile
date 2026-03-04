FROM rust:1-bookworm

# Install Zig toolchain and the aarch64 development headers for Slint
RUN dpkg --add-architecture arm64 
    && apt-get update 
    && apt-get install -y --no-install-recommends 
        ca-certificates 
        pkg-config 
        gcc-aarch64-linux-gnu 
        libc6-dev-arm64-cross 
        libfontconfig1-dev:arm64 
        libxkbcommon-dev:arm64 
        libudev-dev:arm64 
        libdrm-dev:arm64 
        libgbm-dev:arm64 
        libinput-dev:arm64 
        libegl1-mesa-dev:arm64 
        libgles2-mesa-dev:arm64 
        zig 
    && rm -rf /var/lib/apt/lists/*

ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig
ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig

WORKDIR /project

CMD ["cargo", "zigbuild", "--target", "aarch64-unknown-linux-gnu", "--release"]
