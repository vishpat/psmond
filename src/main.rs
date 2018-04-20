extern crate mio;
extern crate mio_uds;
extern crate tempdir;

use std::process::Command;
use mio_uds::UnixListener;
use mio::{Events, Poll, PollOpt, Ready, Token};

use tempdir::TempDir;

struct PerfData {
    mem_total: u32,
    mem_cnt: u32,
    cpu_total: u32,
    cpu_cnt: u32,
}

static sock_name: &'static str = "psmonitor.sock";

fn sample_ps() {
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
    ps_aux_trimmed
        .iter()
        .take(5)
        .for_each(|x| println!("{} {} {}", x.0, x.1, x.2));
}

fn main() {
    let tmp_dir = TempDir::new(&sock_name).expect("Unable to temp file for the socket");
    let addr = tmp_dir.path().join("sock");

    let poll = Poll::new().expect("Unable to create an event poll");
    let srv = UnixListener::bind(&addr).expect("Unable to create the stream socket");
    poll.register(&srv, Token(0), Ready::all(), PollOpt::edge())
        .expect("Unable to register the server");

    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).expect("Unable to get events");
        for event in &events {
            println!("Got an event {:?} !!!", event);
        }
    }
}
