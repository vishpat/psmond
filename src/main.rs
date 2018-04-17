extern crate error_chain;

use std::process::Command;

fn main() {
    let output = Command::new("ps")
        .arg("aux")
        .output()
        .expect("Unable to run ps");

    if !output.status.success() {
        println!("Problem getting the ps output");
        return;
    }

    type PsLine<'a> = (&'a str, u32, u32);

    let ps_output = String::from_utf8(output.stdout).expect("Unable to ps output");

    let mut ps_aux = ps_output
        .lines()
        .skip(1)
        .map(|x| x.split_whitespace().collect::<Vec<&str>>())
        .map(|v| {
            (
                v.get(10).map_or("", |x| x),
                v.get(2).map_or(0, |x| x.parse().unwrap_or(0)),
                v.get(3).map_or(0, |x| x.parse().unwrap_or(0)),
            )
        })
        .collect::<Vec<PsLine>>();


    ps_aux.sort_by(|a, b| a.1.cmp(&b.1));
    ps_aux.sort_by(|a, b| a.2.cmp(&b.2));
    println!("{:?}", ps_aux);

}
