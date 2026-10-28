use node_agent::sched::{simulate, trace};
fn main() {
    let jobs = trace(400, 42, 16);
    let runs = [
        simulate(&jobs, 4, 8, false, None, true),
        simulate(&jobs, 4, 8, true, None, true),
        simulate(&jobs, 4, 8, true, Some((2, 1500)), false),
        simulate(&jobs, 4, 8, true, Some((2, 1500)), true),
    ];
    let rows: Vec<String> = runs.iter().map(|r| format!(
        "{{\"policy\":\"{}\",\"makespan\":{},\"avgWait\":{:.1},\"p95Wait\":{},\"utilization\":{:.3},\"jobs\":{},\"killedJobs\":{}}}",
        r.policy, r.makespan, r.avg_wait, r.p95_wait, r.utilization, r.jobs, r.killed_on_failure)).collect();
    println!("{{\"simulated\":true,\"cluster\":\"4 nodes x 8 GPUs (simulated)\",\"trace\":\"400 jobs, seed 42\",\"runs\":[{}]}}", rows.join(","));
}
