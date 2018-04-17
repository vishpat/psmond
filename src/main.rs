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
