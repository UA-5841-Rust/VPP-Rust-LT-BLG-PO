# Reproducible VPP + Rust bench

Linux root privileges are required for namespaces, AF_PACKET and profiling.
Use a disposable lab/WSL environment. Names `rc-a`, `rc-b`, `rc-*-vpp`,
and `/tmp/rc-week4` are reserved for this bench. Setup re-creates only those
resources; teardown verifies process command lines before stopping processes.
The repository and build artifacts can reside on `/mnt/c`; runtime Unix sockets
must reside on a Linux filesystem.

## Prerequisites

An existing VPP **source checkout and matching release/debug builds**,
Rust/Cargo, clang, Python 3, iproute2, ethtool, iperf3 and perf are needed.
No Python packages are needed by `bench/`. For this run VPP is `/home/user/vpp`;
override `VPP_DIR` for another checkout. The standalone build matches VPP's
generated headers and does not change the week3 plugin symlinks.

On WSL, `/usr/bin/perf` may be a wrapper that cannot find Microsoft kernel
tools. Set `PERF` to the real installed executable under
`/usr/lib/linux-tools/<version>/perf`. Check it with `perf stat -- true`.
Build as a normal user; execute the bench as root.

```bash
bash bench/build.sh release
bash bench/build.sh debug
sudo -E bash bench/validate.sh
sudo -E bash bench/matrix.sh my-run
sudo -E bash bench/mitigation.sh my-fix
python3 bench/summarize.py
```

WSL equivalent for running scripts without a sudo password prompt:

```powershell
wsl -d Ubuntu-24.04 -u root -- bash -lc 'cd /mnt/c/Users/user/Desktop/RUST/week4/VPP-Rust-LT-BLG-PO && bash bench/matrix.sh my-run'
```

Use a new run tag each time; existing result directories are never silently
overwritten by `run.sh`. Snapshot directories can safely be refreshed.

## Topology and individual runs

```text
rc-a (10.44.0.1) -- veth -- host-rc-a-vpp
                              VPP device-input: rust-classify-node
                              -> rust-classify-forward -> interface-output
rc-b (10.44.0.2) -- veth -- host-rc-b-vpp
```

UDP is classified on ingress A and forwarded unchanged to B. ARP, TCP iperf3
control traffic and ICMP follow the normal bidirectional L2 cross-connect;
return traffic on B bypasses the classifier. Both endpoints share a subnet.
The feature does not route IP, change TTL or rewrite MACs in bench mode.

```bash
sudo -E bash bench/setup.sh 1 1 pinned  # workers, AF_PACKET RX queues, affinity
sudo -E bash bench/sanity.sh          # small external UDP + trace + sink assertion
sudo -E bash bench/snapshot.sh bench/results/manual-idle
sudo -E bash bench/run.sh iperf classify 50000000 manual-classify
sudo -E bash bench/run.sh iperf passthrough 50000000 manual-passthrough
sudo -E bash bench/run.sh custom classify 20000 manual-custom
sudo -E bash bench/teardown.sh
```

`matrix.sh` enforces sanity before load and snapshots an idle baseline after
clearing runtime/interface/error counters. It runs both generators at three
rates, both classifier modes, 1 worker/2 queues and 2 workers/2 queues, then
removes VPP thread affinity for a high-load comparison. All load runs are 10 s
by default (`DURATION` overrides this). Payload is 512 bytes. The generator is
on CPU 6; pinned VPP uses main CPU 0 and worker CPUs 2 and 4 (separate physical
cores on the measured machine). These CPU numbers must be adapted for other
machines with fewer than 7 available CPUs. `pinned` is affinity, not exclusive
host-core ownership; no kernel boot configuration is changed.

`setup.sh` explicitly sets TX queue count and puts interface names last in
RX-placement commands to work around CLI integer parsing behavior in this VPP
build. Always inspect the recorded `interface-rx-placement.txt` result.

`validate.sh` injects six external raw Ethernet frames: valid UDP, IPv4 options,
unsupported EtherType, fragment, truncated payload, invalid UDP length. It
asserts exact byte delivery (2 classify / 6 passthrough), drop counters and
records mode/egress in traces. Capture excludes sink-generated outgoing ICMP.

## Artifacts and meanings

Each run has generator/client and sink JSON; before/after VPP CLI snapshots;
stats-segment dump; veth/namespace counters; ethtool ring/queue results;
softnet and per-thread context/affinity data; and perf stat counts. Namespace
`/proc/net/snmp` snapshots expose UDP `RcvbufErrors` for drop localization.
`show errors` includes informational VPP events as well as actual drops:
`forwarded_ok` and AF_PACKET block retirement events are not packet loss.
Multi-worker classification counters are summed across thread sections.
Full stats dumps are losslessly compressed as `stats.txt.gz` to keep large
zero-filled per-thread arrays out of textual review diffs. Read with
`gzip -dc path/to/stats.txt.gz`; no counters are removed.

`iperf` rates are payload bits/s, custom rates are packets/s. Achieved values
are measured from send results, not assumed equal to requested rates. iperf's
receiver `packets` includes missing sequence numbers; `summary.json` subtracts
`lost_packets` when reporting actual received datagrams. Custom loss comes
from successfully sent datagrams minus received datagrams after drain time.
Neither tool measures RTT; iperf jitter is inter-arrival variation.

The highest classifying iperf run records 99 Hz CPU-clock stacks:

```bash
sudo "$PERF" report -f --stdio -i bench/results/my-run-iperf-classify-1000000000/perf.data
sudo "$PERF" script -f -i bench/results/my-run-iperf-classify-1000000000/perf.data > /tmp/stacks.txt
python3 bench/flamegraph.py /tmp/stacks.txt bench/results/my-run-iperf-classify-1000000000/flamegraph.svg
```

The SVG is an actual sample-count flame graph with hover labels; `.folded`
stacks can also be imported into a flame-graph viewer. Open `perf.data` in
Hotspot for interactive call-tree/flamegraph exploration on Linux. Raw binary
profiles and expanded stacks are ignored by Git; report text, folded stacks,
SVG, CLI/JSON evidence and measurements CSV are retained. `-f` permits reading
root-created captures on WSL's Windows mount.

`mitigation.sh` alternates default and `iperf3 -w 212992` socket settings three
times at 1 Gbit/s. Linux doubles the requested socket buffer; actual values
are in JSON. It changes socket options, not global host sysctls. The report
states the outcome and limitations rather than assuming an improvement.
