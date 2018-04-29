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
    let timer_psmap = Arc::new(RwLock::new(psmap));
    let mut total_samples: usize = 0;

    let mut core = Core::new().expect("Unable to create tokio core");

    let timer_task = Interval::new(Instant::now(), Duration::from_millis(1000))
        .for_each(|_instant| {
            let mut psmap = timer_psmap.write().unwrap();
            procstats::sample_ps(&mut psmap, MAX_PROCESSES, &mut total_samples);
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
                let json_response =
                    serde_json::to_string(&psmap.deref()).expect("Unable to serialize the ps map");
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
