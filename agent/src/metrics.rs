use crate::gpu::GpuSample;
use crate::health::Verdict;

/// Prometheus text exposition format.
pub fn render(node: &str, samples: &[GpuSample], v: &Verdict) -> String {
    let mut s = String::new();
    let fams: [(&str, &str, fn(&GpuSample) -> f64); 5] = [
        ("gpu_utilization_percent", "GPU utilization", |g| g.util_pct),
        ("gpu_memory_used_mib", "GPU memory used", |g| g.mem_used_mib),
        ("gpu_temperature_celsius", "GPU temperature", |g| g.temp_c),
        ("gpu_power_watts", "GPU power draw", |g| g.power_w),
        ("gpu_ecc_uncorrected_total", "Uncorrectable ECC errors", |g| g.ecc_uncorrected as f64),
    ];
    for (name, help, f) in fams {
        s += &format!("# HELP {name} {help}\n# TYPE {name} gauge\n");
        for g in samples { s += &format!("{name}{{node=\"{node}\",gpu=\"{}\"}} {}\n", g.index, f(g)); }
    }
    let state = match v { Verdict::Healthy => 0, Verdict::Degraded(_) => 1, Verdict::Drain(_) => 2 };
    s += &format!("# HELP node_health_state 0=healthy 1=degraded 2=drain\n# TYPE node_health_state gauge\nnode_health_state{{node=\"{node}\"}} {state}\n");
    s
}
