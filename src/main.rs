extern crate mio;
extern crate mio_uds;

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use std::path::Path;
use std::path::PathBuf;
use mio_uds::UnixListener;
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::timer::Timer;

struct PerfData {
    cpu_total: f32,
    cpu_cnt: u32,
    mem_total: f32,
    mem_cnt: u32,
}

const TIMER_TOKEN: Token = Token(1);
const SOCK_TOKEN: Token = Token(2);
const MAX_PROCESSES: usize = 5;

static SOCK_FILE: &'static str = "/tmp/psmonitor.sock";

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
    if Path::new(SOCK_FILE).exists() {
        std::fs::remove_file(SOCK_FILE);
    }
    let addr = PathBuf::from(SOCK_FILE);

    let poll = Poll::new().expect("Unable to create an event poll");

    let srv = UnixListener::bind(&addr).expect("Unable to create the stream socket");
    poll.register(&srv, SOCK_TOKEN, Ready::all(), PollOpt::edge())
        .expect("Unable to register the server");

    let mut timer = Timer::default();
    timer.set_timeout(Duration::from_secs(1), 0);
    poll.register(&timer, TIMER_TOKEN, Ready::all(), PollOpt::edge())
        .expect("Unable to register the timer");

    let mut psmap: HashMap<String, PerfData> = HashMap::new();
    let mut total_samples: usize = 0;
    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).expect("Unable to get events");
        for event in &events {
            match event.token() {
                TIMER_TOKEN => {
                    timer.set_timeout(Duration::from_secs(1), 0);
                    sample_ps(&mut psmap, &mut total_samples);
                }
                SOCK_TOKEN => dump_ps_map(&psmap, total_samples),
                _ => panic!("Unexpected event !!"),
            }
        }
    }
}
