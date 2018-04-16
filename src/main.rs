extern crate error_chain;

use std::process::Command;

fn main() {
    let output = Command::new("ps")
                                    .arg("aux")
                                    .output()
                                    .expect("Unable to run ps");

    if !output.status.success() {
        println!("Problem getting the ps output");
    }

    String::from_utf8(output.stdout)
        .expect("Unable to parse output")
        .lines()
        .for_each(|x| println!("{}", x));
}
