"""Losslessly compress existing stats evidence; do not discard any counters."""
import gzip
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent / "results"
    count = 0
    for source in root.glob("**/stats.txt"):
        original = source.read_bytes()
        packed = gzip.compress(original, mtime=0)
        assert gzip.decompress(packed) == original
        source.with_suffix(".txt.gz").write_bytes(packed)
        source.unlink()
        count += 1
    print(f"Losslessly compressed {count} stats dumps")


if __name__ == "__main__":
    main()
