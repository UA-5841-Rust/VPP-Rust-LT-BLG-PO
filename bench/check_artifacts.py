"""Check submitted report links, compressed stats integrity and measured rows."""
import csv
import gzip
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def main():
    bench = Path(__file__).resolve().parent
    for document in (bench / "REPORT.md", bench / "README.md"):
        for link in re.findall(r"\]\(([^)]+)\)", document.read_text()):
            if not link.startswith(("http:", "https:")):
                assert (document.parent / link).exists(), link
    for source in (bench / "results").glob("**/stats.txt.gz"):
        raw = gzip.decompress(source.read_bytes())
        assert b"/sys/heartbeat" in raw and b"/err/rust-classify-node/" in raw, source
    rows = list(csv.DictReader((bench / "results/measurements.csv").open()))
    assert len(rows) == 21, len(rows)
    for row in rows:
        assert int(row["dropped"]) == 0, row
    ET.parse(bench / "results/final-iperf-classify-1000000000/flamegraph.svg")
    print("Report links, 21 measured runs, gzip evidence and flame graph verified.")


if __name__ == "__main__":
    main()
