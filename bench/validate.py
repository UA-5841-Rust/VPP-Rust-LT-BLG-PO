"""External raw-frame integration checks; needs Linux root, no third-party deps."""
import argparse
import collections
import json
import socket
import struct
import subprocess
import time
from pathlib import Path


def frames(src, dst):
    payload = b"week4-integration" * 4
    udp = struct.pack("!HHHH", 4320, 4321, 8 + len(payload), 0) + payload

    def ipv4(options=b""):
        header = bytearray(struct.pack("!BBHHHBBH4s4s", 0x45 + len(options) // 4,
                                     0, 20 + len(options) + len(udp), 1, 0,
                                     64, 17, 0, socket.inet_aton("10.44.0.1"),
                                     socket.inet_aton("10.44.0.2")) + options)
        words = struct.unpack(f"!{len(header) // 2}H", header)
        checksum = sum(words)
        while checksum >> 16:
            checksum = (checksum & 65535) + (checksum >> 16)
        header[10:12] = struct.pack("!H", (~checksum) & 65535)
        return dst + src + b"\x08\x00" + header + udp

    valid = ipv4()
    ethertype = bytearray(valid)
    ethertype[12:14] = b"\x88\xb5"
    fragment = bytearray(valid)
    fragment[20] = 0x20
    length = bytearray(valid)
    length[38:40] = b"\x00\x07"
    return [valid, ipv4(b"\x01\x01\x00\x00"), bytes(ethertype),
            bytes(fragment), valid[:-1], bytes(length)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("role", choices=["check", "capture"])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runtime", default="/tmp/rc-week4")
    parser.add_argument("--vppctl")
    args = parser.parse_args()
    if args.role == "capture":
        sock = socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.htons(3))
        sock.bind(("rc-b-ns", 0))
        sock.settimeout(0.1)
        captured = []
        end = time.monotonic() + 2
        while time.monotonic() < end:
            try:
                packet, address = sock.recvfrom(65536)
                if address[2] != socket.PACKET_OUTGOING and b"week4-integration" in packet:
                    captured.append(packet.hex())
            except socket.timeout:
                pass
        args.output.write_text(json.dumps(captured, indent=2) + "\n")
        return

    def cli(command):
        return subprocess.check_output([args.vppctl, "-s", args.runtime + "/cli.sock",
                                        command], text=True)

    def mac(namespace, interface):
        value = subprocess.check_output(["ip", "netns", "exec", namespace, "cat",
                                         f"/sys/class/net/{interface}/address"], text=True)
        return bytes.fromhex(value.strip().replace(":", ""))

    packets = frames(mac("rc-a", "rc-a-ns"), mac("rc-b", "rc-b-ns"))
    args.output.mkdir(parents=True, exist_ok=True)
    for mode in ("classify", "passthrough"):
        cli("rust classify host-rc-a-vpp to host-rc-b-vpp" +
            (" passthrough" if mode == "passthrough" else ""))
        cli("clear errors")
        cli("clear trace")
        cli("trace add af-packet-input 20")
        capture = args.output / f"{mode}-capture.json"
        process = subprocess.Popen(["ip", "netns", "exec", "rc-b", "python3",
                                    str(Path(__file__).resolve()), "capture",
                                    "--output", str(capture)])
        time.sleep(.3)
        code = "import socket,json,sys,time; s=socket.socket(socket.AF_PACKET,socket.SOCK_RAW); s.bind(('rc-a-ns',0)); [(s.send(bytes.fromhex(p)),time.sleep(.05)) for p in json.loads(sys.argv[1])]"
        subprocess.run(["ip", "netns", "exec", "rc-a", "python3", "-c", code,
                        json.dumps([p.hex() for p in packets])], check=True)
        assert process.wait(timeout=5) == 0
        received = json.loads(capture.read_text())
        expected = packets[:2] if mode == "classify" else packets
        errors = cli("show errors")
        (args.output / f"{mode}-errors.txt").write_text(errors)
        (args.output / f"{mode}-trace.txt").write_text(cli("show trace"))
        assert collections.Counter(received) == collections.Counter(p.hex() for p in expected), mode
        counts = {}
        for line in errors.splitlines():
            fields = line.split()
            if len(fields) >= 3 and fields[1] == "rust-classify-node":
                counts[fields[2]] = counts.get(fields[2], 0) + int(fields[0])
        assert counts.get("forwarded_ok", 0) == len(expected), counts
        assert counts.get("dropped", 0) == (4 if mode == "classify" else 0), counts
        print(f"{mode}: exact external frame delivery and counters passed")
    cli("clear trace")
    cli("rust classify host-rc-a-vpp to host-rc-b-vpp")


if __name__ == "__main__":
    main()
