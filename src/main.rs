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

    type PsLine<'a> = (&'a str, &'a str, &'a str);

    String::from_utf8(output.stdout)
        .expect("Unable to parse output")
        .lines()
        .skip(1)
        .map(|x| x.split_whitespace().collect::<Vec<&str>>())
        .map(|v| {
            (
                v.get(10).map_or("", |x| x),
                v.get(2).map_or("", |x| x),
                v.get(3).map_or("", |x| x),
            )
        })
        .collect::<Vec<PsLine>>()
        .sort_by_key(|x| x.1);
}
