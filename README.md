# GPU Cluster Home Lab (SIMULATED)

> **This is a simulated cluster.** There are no physical GPUs and no live Kubernetes or Slurm cluster behind it. GPU readings come from a seeded simulator with fault injection. The Rust node agent, health rules, scheduler simulation and the Kubernetes/Slurm configs are real code that would run on a real GPU host (`NODE_AGENT_SOURCE=nvidia-smi`).

**Dashboard:** https://gpu-cluster-lab.braylo777.workers.dev (raw: `/sim-report.json`)

## What's here
- `agent/`: Rust node agent (std only, no crates)
  - `gpu.rs`: `nvidia-smi` CSV parser + `SimulatedGpus` source (inject an overheating GPU or ECC errors)
  - `health.rs`: drain rules (any uncorrectable ECC, ≥ 88 °C, no GPUs visible), degrade on ≥ 83 °C or memory ≥ 97%; emits the `scontrol … state=drain` / `kubectl cordon` command (dry-run)
  - `metrics.rs`: Prometheus exposition; `bin/node_agent.rs` serves `/metrics` and `/healthz` on :9400
  - `sched.rs`: Slurm-style FIFO vs. EASY-backfill scheduler on 4 nodes × 8 GPUs with best-fit placement and a node-failure scenario, with and without the agent
- `k8s/node-agent.yaml`: DaemonSet (GPU nodes only, readiness on `/healthz`, read-only root FS) + an `nvidia-smi` smoke pod
- `slurm/`: `slurm.conf` (backfill, cons_tres, GRES), `gres.conf`, and a `HealthCheckProgram` that drains a node when the agent reports 503

## Simulation results (400-job trace, seed 42)
| Scenario | Makespan | Avg wait | GPU util | Jobs killed |
| --- | --- | --- | --- | --- |
| FIFO | 10,614 | 3,249 | 85.3% | 0 |
| EASY backfill | 9,869 | 2,252 | 91.7% | 0 |
| Backfill + silent node failure, no agent | 14,249 | 3,946 | 64.4% | 3,676 |
| Backfill + node failure, agent drains | 12,922 | 3,240 | 71.0% | 2 |

Backfill cuts average wait 31% vs FIFO. When a node silently fails, the agent's drain keeps the scheduler from feeding it work: 2 killed jobs instead of 3,676, and a 9% shorter makespan.

## Run
```bash
cd agent
cargo test                  # 13 tests
cargo run --bin simulate    # reproduces sim-report.json
cargo run --bin node-agent  # curl localhost:9400/metrics
```
See `DEPLOYS.md` for the dashboard deploy and rollback point.
