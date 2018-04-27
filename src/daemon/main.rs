extern crate daemonize;
extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_uds;

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use std::path::Path;
use std::path::PathBuf;
use std::fs::File;
use std::time::Instant;

use tokio::prelude::*;
use tokio::timer::Interval;
use tokio::io::read_to_end;
use tokio_core::reactor::Core;
use tokio_uds::UnixListener;
use daemonize::Daemonize;

struct PerfData {
    cpu_total: f32,
    cpu_cnt: u32,
    mem_total: f32,
    mem_cnt: u32,
}

const MAX_PROCESSES: usize = 5;

static PID_FILE: &'static str = "/tmp/psmonitor.pid";
static SOCK_FILE: &'static str = "/tmp/psmonitor.sock";
static STDOUT_FILE: &'static str = "/tmp/psmonitor.stdout";
static STDERR_FILE: &'static str = "/tmp/psmonitor.stderr";

fn sample_ps(psmap: &mut HashMap<String, PerfData>, total_samples: &mut usize) {
    let output = Command::new("ps")
        .arg("aux")
        .output()
        .expect("Unable to run ps");

    if !output.status.success() {
        println!("Problem getting the ps output");
        return;
    }

    type PsLine<'a> = (&'a str, f32, f32);

    let ps_aux = String::from_utf8(output.stdout).expect("Unable to ps output");

    let mut ps_aux_trimmed = ps_aux
        .lines()
        .skip(1)
        .map(|x| x.split_whitespace().collect::<Vec<&str>>())
        .map(|v| {
            (
                v.get(10).map_or("", |x| x),
                v.get(2).map_or(0.0, |x| x.parse().unwrap_or(0.0)),
                v.get(3).map_or(0.0, |x| x.parse().unwrap_or(0.0)),
            )
        })
        .collect::<Vec<PsLine>>();

    ps_aux_trimmed.sort_by(|a, b| (a.1 as u32).cmp(&(b.1 as u32)).reverse());
    ps_aux_trimmed.sort_by(|a, b| (a.2 as u32).cmp(&(b.2 as u32)).reverse());
    ps_aux_trimmed.iter().take(MAX_PROCESSES).for_each(|x| {
        let perf_data = psmap.entry(x.0.to_string()).or_insert(PerfData {
            cpu_total: x.1,
            cpu_cnt: 1,
            mem_total: x.2,
            mem_cnt: 1,
        });

        perf_data.cpu_total += x.1;
        perf_data.cpu_cnt += 1;
        perf_data.mem_total += x.2;
        perf_data.mem_cnt += 1;
    });

    *total_samples += 1;
}

fn dump_ps_map(psmap: &HashMap<String, PerfData>, total_samples: usize) {
    psmap.iter().for_each(|(k, v)| {
        println!(
            "{} {} {} {} {}",
            k,
            v.cpu_total / v.cpu_cnt as f32,
            v.cpu_total / total_samples as f32,
            v.mem_total / v.mem_cnt as f32,
            v.mem_total / total_samples as f32
        )
    });
}

fn main() {
    let stdout = File::create(STDOUT_FILE).expect("Unable to created stdout file for the daemon");
    let stderr = File::create(STDERR_FILE).expect("Unable to created stderr file for the daemon");
    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        .stdout(stdout)
        .stderr(stderr);

    daemonize.start().expect("Unable to daemonize the process");

    let mut core = Core::new().expect("Unable to create tokio core");

    let mut psmap: HashMap<String, PerfData> = HashMap::new();
    let mut total_samples: usize = 0;

    let timer_task = Interval::new(Instant::now(), Duration::from_millis(1000))
        .for_each(|instant| {
            sample_ps(&mut psmap, &mut total_samples);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    let handle = core.handle();

    let cmd_listener =
        UnixListener::bind(SOCK_FILE, &handle).expect("Unable to bind the Unix socket stream");

    let cmd_task = cmd_listener
        .incoming()
        .for_each(|(socket, _)| {
            let buf = Vec::new();
            let reader = tokio::io::read_to_end(socket, buf)
                .map(|(_, _buf)| {
                    println!("incoming: {:?}", String::from_utf8(_buf).unwrap());
                })
                .then(|_| Ok(()));
            handle.spawn(reader);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    let async_tasks = timer_task.join(cmd_task);

    core.run(async_tasks).expect("Core run failed");
}
