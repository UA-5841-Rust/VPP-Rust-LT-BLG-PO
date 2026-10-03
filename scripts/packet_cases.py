"""Shared Scapy fixtures; mutate serialized bytes so Scapy cannot repair errors."""
from scapy.all import Ether, IP, UDP, Raw


def cases(src="02:00:00:00:00:01", dst="02:00:00:00:00:02"):
    def udp(options=b""):
        return bytes(
            Ether(src=src, dst=dst)
            / IP(src="192.0.2.1", dst="192.0.2.2", options=options)
            / UDP(sport=1234, dport=4321)
            / Raw(b"rust-vpp" * 8)
        )

    valid = udp()

    def changed(offset, value):
        data = bytearray(valid)
        data[offset] = value
        return bytes(data)

    return [
        ("valid_udp", valid, "forwarded_ok"),
        ("ipv4_options", udp(b"\x01\x01\x00\x00"), "forwarded_ok"),
        ("invalid_ethertype", changed(12, 0x86), "unsupported_protocol"),
        ("non_udp", changed(23, 6), "unsupported_protocol"),
        ("fragment_mf", changed(20, 0x20), "unsupported_protocol"),
        ("fragment_offset", changed(21, 1), "unsupported_protocol"),
        ("truncated_payload", valid[:-1], "malformed_packet"),
        ("invalid_version", changed(14, 0x65), "malformed_packet"),
        ("invalid_ihl", changed(14, 0x44), "malformed_packet"),
        ("invalid_ip_length", changed(17, 19), "malformed_packet"),
        ("invalid_udp_length", changed(39, 7), "malformed_packet"),
    ]
