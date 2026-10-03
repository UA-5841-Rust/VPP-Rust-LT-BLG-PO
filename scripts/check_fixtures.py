#!/usr/bin/env python3
"""Check Scapy fixture bytes and classification through the real shared C ABI."""
import argparse
import ctypes
import tempfile
from pathlib import Path

from scapy.all import Ether, Raw
from scapy.utils import RawPcapReader, wrpcap
from packet_cases import cases


class Result(ctypes.Structure):
    _fields_ = [
        ("is_valid", ctypes.c_bool),
        ("protocol", ctypes.c_uint8),
        ("dest_port", ctypes.c_uint16),
        ("error_code", ctypes.c_uint32),
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library", type=Path)
    args = parser.parse_args()
    library = ctypes.CDLL(str(args.library.resolve()))
    library.packet_classify.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    library.packet_classify.restype = Result
    assert ctypes.sizeof(Result) == 8
    fixtures = cases()
    packets = []
    for name, data, expected in fixtures:
        # ctypes buffer stays alive and unchanged through the synchronous call.
        buffer = ctypes.create_string_buffer(data)
        result = library.packet_classify(buffer, len(data))
        if result.is_valid:
            category = "forwarded_ok"
            assert (result.protocol, result.dest_port, result.error_code) == (1, 4321, 0)
        else:
            category = ("unsupported_protocol" if result.error_code in (3, 8, 11)
                        else "malformed_packet")
        assert category == expected, (name, category, expected)
        packets.append(Ether(data[:14]) / Raw(data[14:]))
        assert bytes(packets[-1]) == data, name
    with tempfile.TemporaryDirectory() as directory:
        path = str(Path(directory) / "stream.pcap")
        wrpcap(path, packets)
        with RawPcapReader(path) as reader:
            assert [data for data, _ in reader] == [data for _, data, _ in fixtures]
    print(f"Passed: {len(fixtures)} fixtures, PCAP byte preservation, shared-library ABI")


if __name__ == "__main__":
    main()
