use crate::gpu::GpuSample;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict { Healthy, Degraded(Vec<String>), Drain(Vec<String>) }

pub struct Thresholds { pub temp_warn: f64, pub temp_drain: f64, pub ecc_drain: u64, pub mem_warn_pct: f64 }
impl Default for Thresholds { fn default() -> Self { Thresholds { temp_warn: 83.0, temp_drain: 88.0, ecc_drain: 1, mem_warn_pct: 97.0 } } }

/// Node-level decision: any drain reason drains the node (Slurm `scontrol update state=drain` /
/// `kubectl cordon`); warnings only mark it degraded. No samples at all => drain (agent can't see GPUs).
pub fn evaluate(samples: &[GpuSample], t: &Thresholds) -> Verdict {
    if samples.is_empty() { return Verdict::Drain(vec!["no GPUs visible".into()]); }
    let (mut drain, mut warn) = (vec![], vec![]);
    for g in samples {
        if g.ecc_uncorrected >= t.ecc_drain { drain.push(format!("gpu{} uncorrectable ECC={}", g.index, g.ecc_uncorrected)); }
        if g.temp_c >= t.temp_drain { drain.push(format!("gpu{} temp {:.0}C", g.index, g.temp_c)); }
        else if g.temp_c >= t.temp_warn { warn.push(format!("gpu{} temp {:.0}C", g.index, g.temp_c)); }
        if g.mem_total_mib > 0.0 && 100.0 * g.mem_used_mib / g.mem_total_mib >= t.mem_warn_pct { warn.push(format!("gpu{} memory near full", g.index)); }
    }
    if !drain.is_empty() { Verdict::Drain(drain) } else if !warn.is_empty() { Verdict::Degraded(warn) } else { Verdict::Healthy }
}

/// The command the agent would run (dry-run by default; never executed in the simulation).
pub fn drain_command(node: &str, v: &Verdict, scheduler: &str) -> Option<String> {
    match v {
        Verdict::Drain(r) => Some(match scheduler {
            "slurm" => format!("scontrol update nodename={node} state=drain reason=\"{}\"", r.join("; ")),
            _ => format!("kubectl cordon {node}"),
        }),
        _ => None,
    }
}
