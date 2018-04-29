extern crate daemonize;

use std;
use std::fs::File;
use std::path::Path;

static PID_FILE: &'static str = "/tmp/psmonitor.pid";
pub static SOCK_FILE: &'static str = "/tmp/psmonitor.sock";
static STDOUT_FILE: &'static str = "/tmp/psmonitor.stdout";
static STDERR_FILE: &'static str = "/tmp/psmonitor.stderr";

pub fn daemonsize_process() -> Result<(), daemonize::DaemonizeError> {
    // Daemonize this process
    let stdout = File::create(STDOUT_FILE).expect("Unable to created stdout file for the daemon");
    let stderr = File::create(STDERR_FILE).expect("Unable to created stderr file for the daemon");
    let daemonize = daemonize::Daemonize::new()
        .pid_file(PID_FILE)
        .stdout(stdout)
        .stderr(stderr);

    daemonize.start()?;
    Ok(())
}

pub fn clear_stale_files() -> Result<(), std::io::Error> {
    let stale_files = vec![PID_FILE, SOCK_FILE, STDOUT_FILE, STDERR_FILE];
    stale_files
        .iter()
        .filter(|file| Path::new(file).exists())
        .for_each(|file| {
            std::fs::remove_file(file).expect(&format!("Unable to remove file {}", file))
        });
    Ok(())
}