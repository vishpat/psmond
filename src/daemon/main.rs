extern crate daemonize;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_uds;

use std::collections::HashMap;
use std::time::Duration;
use std::fs::File;
use std::time::Instant;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::ops::Deref;
use tokio::prelude::*;
use tokio::timer::Interval;
use tokio_core::reactor::Core;
use tokio_uds::UnixListener;
use daemonize::Daemonize;

mod procstats;

const MAX_PROCESSES: usize = 5;

static PID_FILE: &'static str = "/tmp/psmonitor.pid";
static SOCK_FILE: &'static str = "/tmp/psmonitor.sock";
static STDOUT_FILE: &'static str = "/tmp/psmonitor.stdout";
static STDERR_FILE: &'static str = "/tmp/psmonitor.stderr";

fn main() {
    let stale_files = vec![PID_FILE, SOCK_FILE, STDOUT_FILE, STDERR_FILE];
    stale_files
        .iter()
        .filter(|file| Path::new(file).exists())
        .for_each(|file| {
            std::fs::remove_file(file)
                .expect(format!("Unable to remove the existing file {}", file).as_ref())
        });

    // Daemonize this process
    //    let stdout = File::create(STDOUT_FILE).expect("Unable to created stdout file for the daemon");
    //    let stderr = File::create(STDERR_FILE).expect("Unable to created stderr file for the daemon");
    //    let daemonize = Daemonize::new()
    //        .pid_file(PID_FILE)
    //        .stdout(stdout)
    //        .stderr(stderr);
    //
    //    daemonize.start().expect("Unable to daemonize the process");
    //
    // Timer task to sample the process stats every second
    let mut core = Core::new().expect("Unable to create tokio core");

    let psmap: HashMap<String, procstats::PerfData> = HashMap::new();
    let timer_psmap = Arc::new(RwLock::new(psmap));
    let mut total_samples: usize = 0;

    let timer_task = Interval::new(Instant::now(), Duration::from_millis(1000))
        .for_each(|_instant| {
            let mut psmap = timer_psmap.write().unwrap();
            procstats::sample_ps(&mut psmap, MAX_PROCESSES, &mut total_samples);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    // Task to start a Unix socket stream server to listen for commands
    let handle = core.handle();

    if Path::new(SOCK_FILE).exists() {}

    let cmd_listener =
        UnixListener::bind(SOCK_FILE, &handle).expect("Unable to bind the Unix socket stream");

    let cmd_task = cmd_listener
        .incoming()
        .for_each(|(mut socket, _)| {
            let status_psmap = timer_psmap.clone();
            handle.spawn(future::lazy(move || {
                let mut buf: [u8; 1024] = [0; 1024];
                socket
                    .read(&mut buf)
                    .expect("Problem while reading from the client");
                let psmap = status_psmap.read().unwrap();
                let json_response =
                    serde_json::to_string(&psmap.deref()).expect("Unable to serialize the ps map");
                socket
                    .write(json_response.as_bytes())
                    .expect("Problem while sending response to the client");
                Ok(())
            }));
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    // Run the futures
    let async_tasks = timer_task.join(cmd_task);

    core.run(async_tasks).expect("Core run failed");
}
