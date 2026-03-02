# Partystation

## Project Goal

The goal of this project is to create a "JackBox-adjacent" party game platform that runs on a Raspberry Pi Zero 2 W. The device will be self-contained, requiring only HDMI and power.

## Core Features

- **Automatic Startup:** The application will launch automatically on boot using `systemctl`.
- **Wi-Fi Hotspot:** The Raspberry Pi will create its own Wi-Fi network for players to connect to.
- **Web Interface:** Players will connect to the game via a web browser at `http://partybox.local`.
- **Game Lobby:** The first player to join will choose a game to play.
- **Playful UI:** The user interface will be fun and engaging.
- **Performant & Resource-Conscious:** The application will be optimized to run smoothly on the Raspberry Pi Zero 2 W.
- **Rust-Based:** The entire application will be written in Rust.
