"""Rebuild the measured table from preserved JSON, CLI and perf artifacts."""
import csv
import json
from pathlib import Path


def udp_counters(path):
    if not path.exists():
        return {}
    lines = path.read_text().splitlines()
    for i, line in enumerate(lines):
        if line.startswith("Udp:"):
            return dict(zip(line.split()[1:], map(int, lines[i + 1].split()[1:])))
    return {}


def main():
    root = Path(__file__).resolve().parent / "results"
    rows = []
    for path in sorted(root.iterdir()):
        if not (path / "summary.json").exists():
            continue
        client = json.loads((path / "client.json").read_text())
        summary = json.loads((path / "summary.json").read_text())
        if "end" in client:
            receiver = client["end"]["sum_received"]
            summary["received_packets"] = receiver["packets"] - receiver["lost_packets"]
        clocks = []
        vectors = []
        for line in (path / "after/run.txt").read_text().splitlines():
            if line.startswith("rust-classify-node "):
                fields = line.split()
                clocks.append(float(fields[-2]))
                vectors.append(int(fields[-4]))
        counters = {}
        for line in (path / "after/errors.txt").read_text().splitlines():
            fields = line.split()
            if len(fields) >= 3 and fields[1] == "rust-classify-node":
                counters[fields[2]] = counters.get(fields[2], 0) + int(fields[0])
        perf = {}
        for line in (path / "perf-stat.txt").read_text().splitlines():
            fields = line.split(";")
            if len(fields) >= 3:
                perf[fields[2]] = fields[0]
        before_udp = udp_counters(path / "before/ns-b-snmp.txt")
        after_udp = udp_counters(path / "after/ns-b-snmp.txt")
        row = dict(run=path.name,
                   offered_pps=round(summary.get("offered_pps", summary.get("achieved_pps")), 2),
                   payload_mbps=round(summary.get("offered_payload_bps", summary.get("achieved_payload_bps")) / 1e6, 3),
                   clocks_per_vector=" / ".join(f"{c:g}" for c in clocks),
                   node_vectors=" / ".join(map(str, vectors)),
                   forwarded=counters.get("forwarded_ok", 0),
                   malformed=counters.get("malformed_packet", 0),
                   unsupported=counters.get("unsupported_protocol", 0),
                   dropped=counters.get("dropped", 0),
                   loss_percent=round(summary["loss_percent"], 5),
                   jitter_ms=round(summary["jitter_ms"], 5) if "jitter_ms" in summary else "N/A",
                   context_switches=perf.get("context-switches", "N/A"),
                   migrations=perf.get("cpu-migrations", "N/A"),
                   sink_rcvbuf_errors=after_udp["RcvbufErrors"] - before_udp["RcvbufErrors"]
                   if after_udp and before_udp else "N/A")
        rows.append(row)
    if not rows:
        raise SystemExit("No completed load runs found under bench/results")
    with (root / "measurements.csv").open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    columns = list(rows[0])
    table = ["| " + " | ".join(columns) + " |", "| " + " | ".join(["---"] * len(columns)) + " |"]
    table.extend("| " + " | ".join(map(str, row.values())) + " |" for row in rows)
    (root / "measurements.md").write_text("\n".join(table) + "\n")


if __name__ == "__main__":
    main()
