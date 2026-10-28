use node_agent::{gpu::{GpuSource, NvidiaSmi, SimulatedGpus}, health::{evaluate, drain_command, Thresholds}, metrics::render};
use std::io::{Read, Write};
use std::net::TcpListener;

/// Serves /metrics and /healthz. NODE_AGENT_SOURCE=sim (default here) or nvidia-smi.
fn main() {
    let node = std::env::var("NODE_NAME").unwrap_or_else(|_| "sim-node-0".into());
    let addr = std::env::var("LISTEN").unwrap_or_else(|_| "0.0.0.0:9400".into());
    let real = std::env::var("NODE_AGENT_SOURCE").as_deref() == Ok("nvidia-smi");
    let mut src: Box<dyn GpuSource> = if real { Box::new(NvidiaSmi) } else { Box::new(SimulatedGpus::new(8, 7)) };
    let listener = TcpListener::bind(&addr).expect("bind");
    eprintln!("node-agent on {addr} ({})", if real { "nvidia-smi" } else { "SIMULATED GPUs" });
    for stream in listener.incoming().flatten() {
        let mut s = stream; let mut buf = [0u8; 1024]; let n = s.read(&mut buf).unwrap_or(0);
        let path = String::from_utf8_lossy(&buf[..n]).split_whitespace().nth(1).unwrap_or("/").to_string();
        let samples = src.sample().unwrap_or_default();
        let v = evaluate(&samples, &Thresholds::default());
        let (code, body) = match path.as_str() {
            "/metrics" => ("200 OK", render(&node, &samples, &v)),
            "/healthz" => match &v { node_agent::health::Verdict::Drain(_) => ("503 Service Unavailable", format!("{:?}\nwould run: {}\n", v, drain_command(&node, &v, "slurm").unwrap())), _ => ("200 OK", format!("{v:?}\n")) },
            _ => ("404 Not Found", "not found\n".into()),
        };
        let _ = write!(s, "HTTP/1.1 {code}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{body}", body.len());
    }
}
