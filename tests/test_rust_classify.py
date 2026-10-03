"""VPP make-test integration. Linked into VPP/test by prepare-vpp.sh."""
from collections import Counter
from pathlib import Path
import sys
import unittest

from framework import VppTestCase
from asfframework import VppTestRunner
from scapy.all import Ether, Raw

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from packet_cases import cases


class TestRustClassify(VppTestCase):
    """Rust classification, actual forwarding/drop, counters, and trace."""

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        try:
            cls.create_pg_interfaces([0])
        except Exception:
            cls.tearDownClass()
            raise

    def setUp(self):
        super().setUp()
        self.pg0.admin_up()
        self.vapi.cli("rust classify pg0")

    def tearDown(self):
        try:
            self.vapi.cli("rust classify pg0 disable")
            self.pg0.admin_down()
        finally:
            super().tearDown()

    def counter(self, name):
        return self.statistics.get_err_counter(f"/err/rust-classify-node/{name}")

    def check_stream(self, repetitions):
        fixtures = cases(src=self.pg0.remote_mac, dst=self.pg0.local_mac)
        expected_counts = Counter(category for _, _, category in fixtures)
        before = {name: self.counter(name) for name in expected_counts}
        dropped_before = self.counter("dropped")
        # Decode only Ethernet; preserve the entire IP payload as raw bytes.
        # Ether's type is retained from the wire, so wrpcap gets linktype 1
        # without repairing any deliberately malformed L3/L4 fields.
        packets = [Ether(data[:14]) / Raw(data[14:])
                   for _, data, _ in fixtures] * repetitions
        expected = [data[6:12] + data[:6] + data[12:]
                    for _, data, category in fixtures
                    if category == "forwarded_ok"] * repetitions
        self.pg_enable_capture(self.pg_interfaces)
        self.pg0.add_stream(packets)
        self.pg_start(trace=False)
        received = self.pg0.get_capture(len(expected))
        self.assertEqual(Counter(bytes(p) for p in received), Counter(expected))
        for name, count in expected_counts.items():
            self.assertEqual(self.counter(name) - before[name], count * repetitions)
        dropped = len(fixtures) - expected_counts["forwarded_ok"]
        self.assertEqual(self.counter("dropped") - dropped_before, dropped * repetitions)
        return received

    def test_classification_and_trace(self):
        self.vapi.cli("clear trace")
        self.vapi.cli("trace add pg-input 64")
        self.check_stream(1)
        trace = self.vapi.cli("show trace")
        self.assertIn("rust-classify", trace)
        self.assertIn("protocol 1 port 4321 valid 1 error 0", trace)
        self.assertIn("valid 0", trace)
        self.logger.info("show trace:\n%s", trace)
        self.logger.info("show errors:\n%s", self.vapi.cli("show errors"))

    def test_multiple_frames(self):
        # 1,100 packets exercises more than one VLIB_FRAME_SIZE-sized batch.
        self.check_stream(100)

    def test_disable_bypasses_classifier(self):
        self.vapi.cli("rust classify pg0 disable")
        before = self.counter("forwarded_ok")
        self.pg_enable_capture(self.pg_interfaces)
        self.pg0.add_stream([Ether(cases()[0][1])])
        self.pg_start()
        self.pg0.assert_nothing_captured(remark="echo feature is disabled")
        self.assertEqual(self.counter("forwarded_ok"), before)


if __name__ == "__main__":
    unittest.main(testRunner=VppTestRunner)
