# Partystation

A self-contained party game platform for Raspberry Pi.

## Current State

- [x] Rust-based Axum server with WebSockets
- [x] mDNS discovery (`partystation.local`)
- [x] Lobby system with host identification
- [x] Simple "Fastest Clicker" game
- [x] Playful UI with CSS animations

## Display Setup (Raspberry Pi HDMI)

Since the Pi has no desktop environment, you can use a lightweight X server to show the game on a TV.

1.  Install necessary tools:
    ```bash
    sudo apt install xserver-xorg xinit x11-xserver-utils chromium-browser
    ```
2.  The "TV" view is available at `http://localhost:3000/tv`.
3.  To launch it on boot, create a script `/home/pi/start_ui.sh`:
    ```bash
    #!/bin/bash
    xset s off
    xset -dpms
    xset s noblank
    chromium-browser --kiosk --noerrdialogs --disable-infobars http://localhost:3000/tv
    ```
4.  Make it executable: `chmod +x /home/pi/start_ui.sh`.
5.  Add `exec /home/pi/start_ui.sh` to your `.xinitrc` or use a systemd service.

## Wi-Fi Hotspot Setup (Raspberry Pi)

Creating a Wi-Fi hotspot is an **Operating System** task, not something the Rust application does directly. To set this up on a Raspberry Pi, you have a few options:

### Option A: Using `nmcli` (Easiest on modern Raspberry Pi OS)

1.  Run the following commands:
    ```bash
    sudo nmcli con add type wifi ifname wlan0 mode ap con-name MyHotspot ssid Partystation autoconnect yes
    sudo nmcli con modify MyHotspot 802-11-wireless.band bg
    sudo nmcli con modify MyHotspot 802-11-wireless-security.key-mgmt wpa-psk
    sudo nmcli con modify MyHotspot 802-11-wireless-security.psk "password123"
    sudo nmcli con modify MyHotspot ipv4.method shared
    sudo nmcli con up MyHotspot
    ```

### Option B: Using RaspAP (Web GUI)

1.  Install RaspAP for a full-featured web dashboard to manage your hotspot:
    ```bash
    curl -sL https://install.raspap.com | bash
    ```

### Option C: Manual `hostapd` and `dnsmasq`
This is more complex and involves editing `/etc/dhcpcd.conf`, `/etc/hostapd/hostapd.conf`, and `/etc/dnsmasq.conf`. Recommended only if you need very specific control.

## Local Development (Mac/PC)

When running on your computer, just connect your phone and computer to the same Wi-Fi network. Access the game using your computer's IP address (e.g., `http://192.168.1.50:3000`).

1.  Build the binary: `cargo build --release`.
2.  Copy the binary to `/usr/local/bin/partystation`.
3.  Copy the `public` directory to `/var/lib/partystation/public`.
4.  Install the `systemd` service: `sudo cp partystation.service /etc/systemd/system/partystation.service`.
5.  Enable and start: `sudo systemctl enable --now partystation`.

## Cross-compiling for Raspberry Pi (Zig, no Docker)

1. On your Pi (Bookworm), install dev libs:
   ```bash
   sudo apt-get update
   sudo apt-get install -y libfontconfig1-dev libxkbcommon-dev libudev-dev libdrm-dev libgbm-dev libinput-dev libegl1-mesa-dev libgles2-mesa-dev
   ```

2. From your Mac, sync a **32-bit armhf** sysroot to `~/pi-sysroot` (or set `PI_SYSROOT`). Make sure the Pi OS is 32-bit or has armhf libs installed:
   ```bash
   rsync -avz pi@<pi-ip>:/usr/include ~/pi-sysroot/
   rsync -avz pi@<pi-ip>:/usr/lib/arm-linux-gnueabihf ~/pi-sysroot/usr/lib/
   rsync -avz pi@<pi-ip>:/usr/share/pkgconfig ~/pi-sysroot/usr/share/
   rsync -avz pi@<pi-ip>:/usr/lib/arm-linux-gnueabihf/pkgconfig ~/pi-sysroot/usr/lib/arm-linux-gnueabihf/
   ```
   If you only see `aarch64-linux-gnu` in `/usr/lib` on your Pi, you synced a 64-bit sysroot; install armhf packages or use a 32-bit Pi OS for this armv7 build.

3. Install tools on Mac: `brew install zig`, `cargo install cargo-zigbuild`, `rustup target add armv7-unknown-linux-gnueabihf aarch64-unknown-linux-gnu`.

4. Build (no Docker):
   ```bash
   ./scripts/zigbuild-armv7-local.sh   # auto-picks armv7 if armhf sysroot; aarch64 if only 64-bit sysroot
   # or override explicitly:
   TARGET_TRIPLE=aarch64-unknown-linux-gnu GLIBC_VER=2.38 ./scripts/zigbuild-armv7-local.sh
   ```

The binary will be at `target/<target-triple>/release/partystation`. Run on Pi with `SLINT_BACKEND=linuxkms-noseat ./partystation`.
