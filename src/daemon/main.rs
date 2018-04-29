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
use std::time::Instant;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::ops::Deref;
use tokio::prelude::*;
use tokio::timer::Interval;
use tokio_core::reactor::Core;
use tokio_uds::UnixListener;


mod procstats;
mod daemon;

const MAX_PROCESSES: usize = 5;

fn main() {
    daemon::clear_stale_files().expect("Unable to clear stale files");
    daemon::daemonsize_process().expect("Unable to daemonize the process");

    let psmap: HashMap<String, procstats::PerfData> = HashMap::new();
    let total_samples = Arc::new(RwLock::new(0));
    let timer_psmap = Arc::new(RwLock::new(psmap));

    let mut core = Core::new().expect("Unable to create tokio core");

    let timer_task = Interval::new(Instant::now(), Duration::from_millis(1000))
        .for_each(|_instant| {
            let mut psmap = timer_psmap.write().unwrap();
            let mut samples = total_samples.write().unwrap();
            procstats::sample_ps(&mut psmap, MAX_PROCESSES, &mut samples);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    // Task to start a Unix socket stream server to listen for commands
    let handle = core.handle();

    if Path::new(daemon::SOCK_FILE).exists() {}

    let cmd_listener =
        UnixListener::bind(daemon::SOCK_FILE, &handle).expect("Unable to bind the Unix socket stream");

    let cmd_task = cmd_listener
        .incoming()
        .for_each(|(mut socket, _)| {
            let status_psmap = timer_psmap.clone();
            let status_samples = total_samples.clone();

            handle.spawn(future::lazy(move || {
                let mut buf: [u8; 1024] = [0; 1024];
                loop {
                    match socket.poll_read() {
                        Async::NotReady => continue,
                        Async::Ready(_) => break,
                    }
                }

                socket
                    .read(&mut buf)
                    .expect("Problem while reading from the client");
                let psmap = status_psmap.read().unwrap();
                let samples = status_samples.read().unwrap();

                #[derive(Serialize)]
                struct PsDump<'a> {
                    psmap : &'a HashMap<String, procstats::PerfData>,
                    total_samples: usize,
                }

                let psdump_data = PsDump{psmap: psmap.deref(), total_samples:*(samples.deref())};

                let json_response =
                    serde_json::to_string(&psdump_data).expect("Unable to serialize the ps map");
                socket.write(json_response.as_bytes()).unwrap_or(0);
                socket.flush().unwrap_or(());

                Ok(())
            }));
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    // Run the futures
    let async_tasks = timer_task.join(cmd_task);

    core.run(async_tasks).expect("Core run failed");
}
