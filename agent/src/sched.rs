//! SIMULATED cluster scheduler: Slurm-style FIFO vs. EASY backfill on N nodes x G GPUs,
//! with an optional node failure that the node agent detects and drains.
use crate::rng::Rng;

#[derive(Debug, Clone)]
pub struct Job { pub id: u32, pub submit: u64, pub gpus: u32, pub runtime: u64 }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report { pub policy: String, pub makespan: u64, pub avg_wait: f64, pub p95_wait: u64, pub utilization: f64, pub jobs: usize, pub killed_on_failure: u32 }

pub fn trace(n: usize, seed: u64, max_gpus: u32) -> Vec<Job> {
    let mut r = Rng::new(seed); let mut t = 0;
    (0..n as u32).map(|id| {
        t += r.range(0, 20);
        let gpus = [1, 1, 2, 4, 8, 8, 16][r.range(0, 6) as usize].min(max_gpus);
        Job { id, submit: t, gpus, runtime: r.range(10, 240) }
    }).collect()
}

/// failure: Some((node, at_time)) — node goes bad; with `agent` the node is drained at detection
/// (no new jobs placed; running jobs on it are requeued once). Without the agent the scheduler keeps
/// placing jobs on the bad node and every one of them dies and is requeued.
pub fn simulate(jobs: &[Job], nodes: u32, gpus_per_node: u32, backfill: bool, failure: Option<(u32, u64)>, agent: bool) -> Report {
    let total = nodes * gpus_per_node;
    let mut free = vec![gpus_per_node; nodes as usize];
    let mut queue: Vec<Job> = vec![];
    let mut pending: Vec<Job> = jobs.to_vec(); pending.sort_by_key(|j| j.submit); pending.reverse();
    let mut running: Vec<(u64, Job, Vec<(usize, u32)>)> = vec![]; // (end, job, allocations)
    let (mut t, mut busy, mut waits, mut killed) = (0u64, 0u64, vec![], 0u32);
    let mut drained = vec![false; nodes as usize];
    let mut bad = vec![false; nodes as usize];
    let place = |free: &mut Vec<u32>, drained: &[bool], need: u32| -> Option<Vec<(usize, u32)>> {
        let avail: u32 = free.iter().enumerate().filter(|(i, _)| !drained[*i]).map(|(_, f)| *f).sum();
        if avail < need { return None; }
        let mut left = need; let mut alloc = vec![];
        // best fit: fill fullest nodes first to keep whole nodes free for big jobs
        let mut order: Vec<usize> = (0..free.len()).filter(|i| !drained[*i] && free[*i] > 0).collect();
        order.sort_by_key(|i| free[*i]);
        for i in order { if left == 0 { break; } let take = free[i].min(left); free[i] -= take; left -= take; alloc.push((i, take)); }
        Some(alloc)
    };
    loop {
        while let Some(j) = pending.last() { if j.submit <= t { queue.push(pending.pop().unwrap()); } else { break; } }
        if let Some((n, at)) = failure { if t >= at && !bad[n as usize] {
            bad[n as usize] = true;
            if agent { drained[n as usize] = true; }
            // jobs touching the bad node die and requeue (counted once per kill)
            let (dead, alive): (Vec<_>, Vec<_>) = running.drain(..).partition(|(_, _, a)| a.iter().any(|(i, _)| *i == n as usize));
            running = alive;
            for (_, j, a) in dead { for (i, g) in a { free[i] += g; } killed += 1; queue.insert(0, j); }
        }}
        // start jobs
        let mut i = 0;
        let head_need = queue.first().map(|j| j.gpus);
        let mut shadow: Option<u64> = None;
        while i < queue.len() {
            let j = queue[i].clone();
            if let Some(a) = place(&mut free, &drained, j.gpus) {
                let on_bad = a.iter().any(|(n, _)| bad[*n]);
                if i == 0 || backfill && shadow.map_or(true, |s| t + j.runtime <= s) {
                    waits.push(t - j.submit.min(t));
                    queue.remove(i);
                    if on_bad { // silent failure: job dies after 5 ticks, requeued
                        killed += 1; running.push((t + 5, Job { runtime: 0, ..j.clone() }, a));
                        queue.push(j); waits.pop();
                    } else { busy += j.gpus as u64 * j.runtime; running.push((t + j.runtime, j, a)); }
                    continue;
                }
                for (n, g) in a { free[n] += g; }
            }
            if i == 0 {
                if !backfill { break; }
                // shadow time: when enough GPUs free up for the head job
                let mut ends: Vec<_> = running.iter().map(|(e, j, _)| (*e, j.gpus)).collect(); ends.sort();
                let mut have: u32 = free.iter().enumerate().filter(|(k, _)| !drained[*k]).map(|(_, f)| *f).sum();
                for (e, g) in ends { have += g; if have >= head_need.unwrap_or(0) { shadow = Some(e); break; } }
            }
            i += 1;
        }
        if pending.is_empty() && queue.is_empty() && running.is_empty() { break; }
        // advance to next event
        let next_end = running.iter().map(|r| r.0).min();
        let next_sub = pending.last().map(|j| j.submit);
        let next_fail = failure.and_then(|(n, at)| if !bad[n as usize] && at > t { Some(at) } else { None });
        let nt = [next_end, next_sub, next_fail].into_iter().flatten().min().unwrap_or(t + 1);
        t = nt.max(t + 0);
        let (done, still): (Vec<_>, Vec<_>) = running.drain(..).partition(|r| r.0 <= t);
        running = still;
        for (_, _, a) in done { for (n, g) in a { free[n] += g; } }
        if t > 10_000_000 { break; }
    }
    waits.sort();
    let p95 = if waits.is_empty() { 0 } else { waits[((waits.len() as f64 * 0.95) as usize).min(waits.len() - 1)] };
    Report {
        policy: format!("{}{}", if backfill { "backfill" } else { "fifo" }, match (failure, agent) { (None, _) => "", (Some(_), true) => " + node failure, agent drains", (Some(_), false) => " + node failure, no agent" }),
        makespan: t, avg_wait: waits.iter().sum::<u64>() as f64 / waits.len().max(1) as f64, p95_wait: p95,
        utilization: busy as f64 / (t.max(1) * total as u64) as f64, jobs: jobs.len(), killed_on_failure: killed,
    }
}
