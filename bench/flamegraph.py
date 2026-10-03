"""Render observed perf-script stacks as an SVG flame graph (sample widths)."""
import argparse
import collections
import hashlib
import html
import re
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    stacks = collections.Counter()
    frames = []
    thread = ""
    for line in args.input.read_text().splitlines() + [""]:
        if not line.strip():
            if frames:
                stacks[tuple([thread] + list(reversed(frames)))] += 1
            frames = []
        elif line.startswith("#"):
            continue
        elif not line[0].isspace():
            thread = line.split()[0]
        else:
            match = re.match(r"\s*[0-9a-f]+\s+(.+?)\s+\(", line)
            if match:
                frames.append(re.sub(r"\+0x[0-9a-f]+$", "", match[1]))
    if not stacks:
        raise SystemExit("No usable sampled stacks; cannot create a flame graph")
    folded = args.output.with_suffix(".folded")
    folded.write_text("\n".join(";".join(k) + " " + str(v)
                                for k, v in sorted(stacks.items())) + "\n")
    tree = {}
    total = sum(stacks.values())
    for stack, count in stacks.items():
        node = tree
        for name in stack:
            entry = node.setdefault(name, [0, {}])
            entry[0] += count
            node = entry[1]
    depth = max(map(len, stacks))
    height = depth * 20 + 85
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="{height}" viewBox="0 0 1200 {height}">',
             '<style>text{font-family:monospace;font-size:11px}rect:hover{stroke:black;stroke-width:1}</style>',
             '<rect width="100%" height="100%" fill="white"/>',
             f'<text x="10" y="22">VPP CPU-clock flame graph — {total} observed stack samples (99 Hz)</text>',
             '<text x="10" y="42">Width = sample count; hover for symbol/count. Software sampling, not hardware cycles.</text>']

    def draw(node, x, level):
        for name, (count, children) in sorted(node.items()):
            width = count / total * 1180
            y = height - 30 - level * 20
            shade = int(hashlib.sha256(name.encode()).hexdigest()[:2], 16)
            color = f"rgb(245,{100 + shade // 2},{60 + shade // 3})"
            label = html.escape(name)
            parts.append(f'<g><title>{label}: {count} samples ({count / total:.2%})</title>'
                         f'<rect x="{x:.2f}" y="{y}" width="{width:.2f}" height="19" fill="{color}"/>')
            chars = max(int(width / 7) - 1, 0)
            if chars > 3:
                parts.append(f'<text x="{x + 3:.2f}" y="{y + 14}">{html.escape(name[:chars])}</text>')
            parts.append('</g>')
            draw(children, x, level + 1)
            x += width

    draw(tree, 10, 0)
    parts.append('</svg>')
    args.output.write_text("\n".join(parts))
    print(f"Rendered {total} sampled stacks")


if __name__ == "__main__":
    main()
