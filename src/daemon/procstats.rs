use std::process::Command;
use std::collections::HashMap;

const PRUNE_INTERVAL: usize = 1000;

#[derive(Serialize, Deserialize)]
pub struct PerfData {
    cpu_total: f32,
    mem_total: f32,
    sample_cnt: u32,
}

pub fn sample_ps(
    psmap: &mut HashMap<String, PerfData>,
    max_processes: usize,
    total_samples: &mut usize,
) {
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
    ps_aux_trimmed.iter().take(max_processes).for_each(|x| {
        let perf_data = psmap.entry(x.0.to_string()).or_insert(PerfData {
            cpu_total: x.1,
            mem_total: x.2,
            sample_cnt: 1,
        });

        perf_data.cpu_total += x.1;
        perf_data.mem_total += x.2;
        perf_data.sample_cnt += 1;
    });

    *total_samples += 1;

    if *total_samples % PRUNE_INTERVAL == 0 {
        let purge_keys: Vec<_> = psmap
            .iter()
            .filter(|&(_, perf_data)| {
                ((perf_data.sample_cnt as f32 * 100.0) / (*total_samples as f32) < 5.0)
            })
            .map(|(k, _)| k.clone())
            .collect();

        purge_keys.iter().for_each(|k| {
            psmap.remove(k);
            ()
        });
    }
}
