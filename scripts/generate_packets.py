#!/usr/bin/env python3
"""Generate exact PCAP records and a VPP exec file for manual verification."""
import argparse
from collections import Counter
from pathlib import Path

from scapy.utils import RawPcapWriter
from packet_cases import cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if any(c.isspace() for c in str(output)):
        parser.error("use an output path without whitespace for the VPP CLI file")
    output.mkdir(parents=True, exist_ok=True)
    fixtures = cases()
    commands = [
        "create packet-generator interface pg0",
        "set interface state pg0 up",
        "rust classify pg0",
        "clear errors",
        "clear trace",
        # Trace at the input node so downstream classifier records are emitted.
        "trace add pg-input 64",
    ]
    for name, packet, _ in fixtures:
        path = output / f"{name}.pcap"
        writer = RawPcapWriter(str(path), linktype=1, sync=True)
        writer.write(packet)
        writer.close()
        commands.append(
            "packet-generator new { "
            f"name {name} limit 1 node ethernet-input interface pg0 pcap {path}"
            " }"
        )
    commands.append("packet-generator enable")
    (output / "run.cli").write_text("\n".join(commands) + "\n", encoding="utf-8")
    counts = Counter(category for _, _, category in fixtures)
    print(f"VPP: exec {output / 'run.cli'}")
    print("After streams finish: show errors; show trace; show run")
    print(f"Expected classification counts: {dict(counts)}")
    print(f"Expected dropped: {len(fixtures) - counts['forwarded_ok']}")


if __name__ == "__main__":
    main()
