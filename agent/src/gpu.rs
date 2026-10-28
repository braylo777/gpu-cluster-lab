use crate::rng::Rng;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuSample {
    pub index: u32,
    pub name: String,
    pub util_pct: f64,
    pub mem_used_mib: f64,
    pub mem_total_mib: f64,
    pub temp_c: f64,
    pub power_w: f64,
    pub ecc_uncorrected: u64,
}

pub trait GpuSource { fn sample(&mut self) -> Result<Vec<GpuSample>, String>; }

/// Parses `nvidia-smi --query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,ecc.errors.uncorrected.volatile.total --format=csv,noheader,nounits`
pub fn parse_nvidia_smi(csv: &str) -> Result<Vec<GpuSample>, String> {
    let mut out = vec![];
    for (ln, line) in csv.lines().enumerate() {
        if line.trim().is_empty() { continue; }
        let f: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if f.len() != 8 { return Err(format!("line {}: expected 8 fields, got {}", ln + 1, f.len())); }
        let num = |s: &str| -> Result<f64, String> {
            if s == "[N/A]" || s == "N/A" { Ok(0.0) } else { s.parse::<f64>().map_err(|e| format!("line {}: {e}: {s}", ln + 1)) }
        };
        out.push(GpuSample {
            index: num(f[0])? as u32, name: f[1].to_string(), util_pct: num(f[2])?, mem_used_mib: num(f[3])?,
            mem_total_mib: num(f[4])?, temp_c: num(f[5])?, power_w: num(f[6])?, ecc_uncorrected: num(f[7])? as u64,
        });
    }
    Ok(out)
}

/// Real source: shells out to nvidia-smi (only on GPU hosts).
pub struct NvidiaSmi;
impl GpuSource for NvidiaSmi {
    fn sample(&mut self) -> Result<Vec<GpuSample>, String> {
        let o = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,ecc.errors.uncorrected.volatile.total", "--format=csv,noheader,nounits"])
            .output().map_err(|e| format!("nvidia-smi: {e}"))?;
        parse_nvidia_smi(&String::from_utf8_lossy(&o.stdout))
    }
}

/// SIMULATED source used in this lab (no physical GPUs). Faults can be injected.
pub struct SimulatedGpus { pub n: u32, pub rng: Rng, pub hot_gpu: Option<u32>, pub ecc_gpu: Option<u32> }
impl SimulatedGpus { pub fn new(n: u32, seed: u64) -> Self { SimulatedGpus { n, rng: Rng::new(seed), hot_gpu: None, ecc_gpu: None } } }
impl GpuSource for SimulatedGpus {
    fn sample(&mut self) -> Result<Vec<GpuSample>, String> {
        Ok((0..self.n).map(|i| {
            let util = 40.0 + self.rng.f64() * 55.0;
            GpuSample {
                index: i, name: "SIMULATED H100 80GB".into(), util_pct: util,
                mem_used_mib: 81559.0 * util / 100.0, mem_total_mib: 81559.0,
                temp_c: if self.hot_gpu == Some(i) { 91.0 } else { 45.0 + util * 0.3 },
                power_w: 150.0 + util * 5.0,
                ecc_uncorrected: if self.ecc_gpu == Some(i) { 3 } else { 0 },
            }
        }).collect())
    }
}
