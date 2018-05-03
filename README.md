# psmond
Linux process monitor in RUST

## psmond
psmond is a Linux daemon (written in RUST) that keeps track of the processes consuming high CPU and memory. The daemon can be built and run as follows

<pre>
cargo build --release
./target/release/psmond
</pre>

## psmon
psmon is a (python) client that interacts with the daemon on a Unix domain socket dumps information in json format

<pre>
./psmon | python -m json.tool
{
    "psmap": {
        "/home/vishpat/idea-IC-181.4445.78/jre64/bin/java": {
            "cpu_total": 6768.5147,
            "mem_total": 7833.0,
            "sample_cnt": 1119
        },
        "/opt/google/chrome/chrome": {
            "cpu_total": 5923.9004,
            "mem_total": 8310.233,
            "sample_cnt": 2237
        },
        "/usr/bin/qemu-system-x86_64": {
            "cpu_total": 13987.5,
            "mem_total": 53488.758,
            "sample_cnt": 1119
        },
        "cinnamon": {
            "cpu_total": 81784.98,
            "mem_total": 3916.5,
            "sample_cnt": 1119
        }
    },
    "total_samples": 1118
}

</pre>
