#!/usr/bin/env python3
import unittest
from framework import VppTestCase
from scapy.layers.l2 import Ether
from scapy.layers.inet import IP, UDP
from scapy.packet import Raw

class TestRustClassify(VppTestCase):
    """ Rust Classify Plugin Test Case """

    def setUp(self):
        super(TestRustClassify, self).setUp()
        self.vapi.cli("clear errors")

    def test_rust_classify_node(self):
        """ Test Rust Classification Scenarios """
        
        # 1. Valid UDP packet
        p_valid = (Ether(src="00:11:22:33:44:55", dst="aa:bb:cc:dd:ee:ff"),
                   IP(src="127.0.0.1", dst="127.0.0.1"),
                   UDP(sport=1234, dport=5678),
                   Raw(b'hello'))

        # 2. IPv4 packet with options (IHL > 5)
        p_opts = (Ether(src="00:11:22:33:44:55", dst="aa:bb:cc:dd:ee:ff"),
                  IP(src="127.0.0.1", dst="127.0.0.1", options=['\x00\x00\x00\x00']),
                  UDP(sport=1234, dport=5678),
                  Raw(b'hello'))

        # 3. Packet with invalid EtherType (IPv6 ethertype but IPv4 header)
        p_inv_eth = (Ether(src="00:11:22:33:44:55", dst="aa:bb:cc:dd:ee:ff", type=0x86dd),
                     IP(src="127.0.0.1", dst="127.0.0.1"),
                     UDP(sport=1234, dport=5678),
                     Raw(b'hello'))

        # 4. Truncated UDP payload (Forcing IP len to be larger than actual packet)
        p_trunc = (Ether(src="00:11:22:33:44:55", dst="aa:bb:cc:dd:ee:ff"),
                   IP(src="127.0.0.1", dst="127.0.0.1", len=100),
                   UDP(sport=1234, dport=5678, len=80),
                   Raw(b'short'))

        # Inject packets directly into our node using Scapy's hex conversion
        scenarios = [
            ("valid", p_valid),
            ("options", p_opts),
            ("invalid_eth", p_inv_eth),
            ("truncated", p_trunc)
        ]

        for name, pkt in scenarios:
            hex_pkt = bytes(pkt).hex()
            self.vapi.cli(f"packet-generator new {{ name {name} limit 1 node rust-classify-node data {{ hex 0x{hex_pkt} }} }}")

        # Run traffic
        self.vapi.cli("packet-generator enable")

        # Fetch errors and log them to test output
        err_output = self.vapi.cli("show errors")
        self.logger.info("--- SHOW ERRORS OUTPUT ---")
        self.logger.info(err_output)
        
        # Verify that our node processed the packets
        self.assertIn("rust-classify-node", err_output)

if __name__ == '__main__':
    unittest.main()