"""Fixed payload UDP load; receiver counts application datagrams, not NIC packets."""
import argparse
import json
import signal
import socket
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("role", choices=["send", "receive"])
    parser.add_argument("--pps", type=int, default=10000)
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--size", type=int, default=512)
    parser.add_argument("--flows", type=int, default=8)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if min(args.pps, args.seconds, args.size, args.flows) <= 0:
        parser.error("All numeric parameters must be positive")
    if args.role == "receive":
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 4 * 1024 * 1024)
        sock.bind(("10.44.0.2", 5202))
        sock.settimeout(0.2)
        running = True

        def stop(_signal, _frame):
            nonlocal running
            running = False

        signal.signal(signal.SIGTERM, stop)
        packet = bytearray(65536)
        count = total = 0
        start = time.monotonic()
        while running:
            try:
                total += sock.recv_into(packet)
                count += 1
            except socket.timeout:
                continue
        result = {"received_packets": count, "received_bytes": total,
                  "receiver_seconds": time.monotonic() - start}
    else:
        sockets = [socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                   for _ in range(args.flows)]
        for sock in sockets:
            sock.connect(("10.44.0.2", 5202))
        payload = bytes(args.size)
        count = errors = 0
        start = time.monotonic()
        end = start + args.seconds
        # Batches reduce clock calls; no per-packet payload allocation.
        while time.monotonic() < end:
            target = min(int((time.monotonic() - start) * args.pps) + 1,
                         int(args.seconds * args.pps))
            batch = min(max(target - count, 0), 64)
            if batch == 0:
                time.sleep(0.0001)
                continue
            for _ in range(batch):
                try:
                    sockets[count % args.flows].send(payload)
                    count += 1
                except OSError:
                    errors += 1
        elapsed = time.monotonic() - start
        result = {"sent_packets": count, "send_errors": errors,
                  "seconds": elapsed, "achieved_pps": count / elapsed,
                  "achieved_payload_bps": count * args.size * 8 / elapsed,
                  "requested_pps": args.pps, "payload_bytes": args.size,
                  "flows": args.flows}
    with open(args.output, "w", encoding="utf-8") as output:
        json.dump(result, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
