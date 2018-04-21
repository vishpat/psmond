extern crate mio;
extern crate mio_uds;
extern crate tempdir;

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use mio_uds::UnixListener;
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::timer::Timer;

use tempdir::TempDir;

struct PerfData {
    mem_total: f32,
    mem_cnt: u32,
    cpu_total: f32,
    cpu_cnt: u32,
}

const TIMER_TOKEN: Token = Token(1);
const SOCK_TOKEN: Token = Token(2);
const MAX_PROCESSES: usize = 5;

static sock_name: &'static str = "psmonitor.sock";

fn sample_ps(psmap: &mut HashMap<String, PerfData>) {
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
            cpu_cnt: 1,
            cpu_total: x.1,
            mem_cnt: 1,
            mem_total: x.2,
        });

        perf_data.cpu_cnt += 1;
        perf_data.mem_cnt += 1;
        perf_data.cpu_total += x.1;
        perf_data.mem_total += x.2;
    });
}

fn main() {
    let tmp_dir = TempDir::new(&sock_name).expect("Unable to temp file for the socket");
    let addr = tmp_dir.path().join("sock");

    let poll = Poll::new().expect("Unable to create an event poll");

    let srv = UnixListener::bind(&addr).expect("Unable to create the stream socket");
    poll.register(&srv, SOCK_TOKEN, Ready::all(), PollOpt::edge())
        .expect("Unable to register the server");

    let mut timer = Timer::default();
    timer.set_timeout(Duration::from_secs(1), 0);
    poll.register(&timer, TIMER_TOKEN, Ready::all(), PollOpt::edge())
        .expect("Unable to register the timer");

    let mut psmap: HashMap<String, PerfData> = HashMap::new();

    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).expect("Unable to get events");
        for event in &events {
            match event.token() {
                TIMER_TOKEN => {
                    timer.set_timeout(Duration::from_secs(1), 0);
                    sample_ps(&mut psmap);
                }
                SOCK_TOKEN => {}
                _ => panic!("Unexpected event !!"),
            }
        }
    }
}
