use node_agent::gpu::*;
use node_agent::health::*;
use node_agent::metrics::render;
use node_agent::sched::*;

const CSV: &str = "0, NVIDIA H100 80GB HBM3, 97, 70000, 81559, 71, 650.5, 0\n1, NVIDIA H100 80GB HBM3, 12, 1000, 81559, 38, 90.1, [N/A]\n";

#[test] fn parses_nvidia_smi_csv() {
    let g = parse_nvidia_smi(CSV).unwrap();
    assert_eq!(g.len(), 2);
    assert_eq!(g[0].util_pct, 97.0);
    assert_eq!(g[1].ecc_uncorrected, 0);
}
#[test] fn rejects_malformed_csv() { assert!(parse_nvidia_smi("0, H100, 1").is_err()); assert!(parse_nvidia_smi("a,b,c,d,e,f,g,h").is_err()); }
#[test] fn healthy_sim_node() {
    let mut s = SimulatedGpus::new(8, 1);
    assert_eq!(evaluate(&s.sample().unwrap(), &Thresholds::default()), Verdict::Healthy);
}
#[test] fn ecc_error_drains() {
    let mut s = SimulatedGpus::new(8, 1); s.ecc_gpu = Some(3);
    match evaluate(&s.sample().unwrap(), &Thresholds::default()) { Verdict::Drain(r) => assert!(r[0].contains("gpu3")), v => panic!("{v:?}") }
}
#[test] fn overheat_drains_and_warm_degrades() {
    let mut s = SimulatedGpus::new(2, 1); s.hot_gpu = Some(0);
    assert!(matches!(evaluate(&s.sample().unwrap(), &Thresholds::default()), Verdict::Drain(_)));
    let mut g = parse_nvidia_smi(CSV).unwrap(); g[0].temp_c = 85.0;
    assert!(matches!(evaluate(&g, &Thresholds::default()), Verdict::Degraded(_)));
}
#[test] fn no_gpus_visible_drains() { assert!(matches!(evaluate(&[], &Thresholds::default()), Verdict::Drain(_))); }
#[test] fn drain_commands() {
    let v = Verdict::Drain(vec!["gpu1 ECC".into()]);
    assert_eq!(drain_command("n1", &v, "slurm").unwrap(), "scontrol update nodename=n1 state=drain reason=\"gpu1 ECC\"");
    assert_eq!(drain_command("n1", &v, "k8s").unwrap(), "kubectl cordon n1");
    assert!(drain_command("n1", &Verdict::Healthy, "slurm").is_none());
}
#[test] fn prometheus_format() {
    let g = parse_nvidia_smi(CSV).unwrap();
    let m = render("n1", &g, &Verdict::Healthy);
    assert!(m.contains("# TYPE gpu_temperature_celsius gauge"));
    assert!(m.contains("gpu_utilization_percent{node=\"n1\",gpu=\"0\"} 97"));
    assert!(m.contains("node_health_state{node=\"n1\"} 0"));
}
#[test] fn trace_is_deterministic_and_bounded() {
    let a = trace(50, 9, 16); let b = trace(50, 9, 16);
    assert_eq!(a.iter().map(|j| j.runtime).collect::<Vec<_>>(), b.iter().map(|j| j.runtime).collect::<Vec<_>>());
    assert!(a.iter().all(|j| j.gpus <= 16 && j.gpus >= 1));
}
#[test] fn all_jobs_finish_and_util_is_valid() {
    let j = trace(200, 3, 16);
    for bf in [false, true] { let r = simulate(&j, 4, 8, bf, None, true); assert!(r.utilization > 0.0 && r.utilization <= 1.0); assert_eq!(r.killed_on_failure, 0); }
}
#[test] fn backfill_beats_fifo_on_wait() {
    let j = trace(400, 42, 16);
    assert!(simulate(&j, 4, 8, true, None, true).avg_wait < simulate(&j, 4, 8, false, None, true).avg_wait);
}
#[test] fn agent_drain_limits_failure_damage() {
    let j = trace(400, 42, 16);
    let no = simulate(&j, 4, 8, true, Some((2, 1500)), false);
    let yes = simulate(&j, 4, 8, true, Some((2, 1500)), true);
    assert!(yes.killed_on_failure < 10 && no.killed_on_failure > 100);
    assert!(yes.makespan < no.makespan);
}
#[test] fn single_big_job_fits_exactly() {
    let j = vec![Job { id: 0, submit: 0, gpus: 32, runtime: 10 }];
    let r = simulate(&j, 4, 8, true, None, true);
    assert_eq!(r.makespan, 10); assert!((r.utilization - 1.0).abs() < 1e-9);
}
