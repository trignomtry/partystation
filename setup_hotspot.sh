#!/bin/bash

# Partystation Hotspot Setup Script (DietPi)
# 1. Install necessary software
sudo dietpi-software install 60 # Hotspot

# 2. Configure Hostapd for OPEN network
cat <<EOF | sudo tee /etc/hostapd/hostapd.conf
interface=wlan0
driver=nl80211
ssid=PartyBox
hw_mode=g
channel=6
auth_algs=1
wmm_enabled=0
EOF

# 3. Configure DNSmasq for partybox.play
sudo apt install dnsmasq -y
cat <<EOF | sudo tee /etc/dnsmasq.conf
interface=wlan0
dhcp-range=192.168.42.2,192.168.42.20,255.255.255.0,24h
address=/partybox.play/192.168.42.1
EOF

# 4. Port 80 -> 3000 Redirect
sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 3000
sudo apt install iptables-persistent -y
sudo netfilter-persistent save

# 5. Restart services
sudo systemctl restart hostapd
sudo systemctl restart dnsmasq

echo "Hotspot 'PartyBox' setup complete! Browse to 'partybox.play'."
