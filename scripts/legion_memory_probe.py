"""Run each isolated Legion mode in a fresh process; measure live heap and OS memory."""
import json
import argparse
import os
from pathlib import Path
import subprocess
from process_metrics import ProcessMetrics
from simulation import ROOT

def run(output_path, modes):
    subprocess.run(["cargo", "build", "-p", "server", "--example", "legion_memory_probe", "--locked", "--offline"], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=ROOT))
    exe = Path(metadata["target_directory"]) / "debug" / "examples" / ("legion_memory_probe.exe" if os.name == "nt" else "legion_memory_probe")
    output = ROOT / output_path
    output.mkdir(parents=True, exist_ok=False)
    records = []
    for mode in modes:
        process = subprocess.Popen([str(exe), mode], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        monitor = ProcessMetrics(process.pid)
        try:
            for line in process.stdout:
                record = json.loads(line)
                record["process_metrics"] = monitor.sample()
                records.append(record)
                process.stdin.write("ack\n")
                process.stdin.flush()
            if process.wait(timeout=30):
                raise RuntimeError(f"Probe failed: {mode}")
        finally:
            monitor.close()
            if process.poll() is None:
                process.kill()
                process.wait()
            process.stdin.close()
            process.stdout.close()
        baseline = next(r for r in records if r["mode"] == mode and r["phase"] == "baseline")
        final = [r for r in records if r["mode"] == mode and r["phase"] == "checkpoint"][-1]
        private_before = baseline["process_metrics"]["private_bytes"]
        private_after = final["process_metrics"]["private_bytes"]
        private_growth = private_after - private_before if private_before is not None and private_after is not None else None
        print(mode, "live heap growth bytes:", final["live_bytes"] - baseline["live_bytes"], "private growth bytes:", private_growth)
    (output / "measurements.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    print(output)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default="workload-results/legion-isolation")
    parser.add_argument("--modes", nargs="+", choices=["direct", "commands", "schedule", "pool", "take", "recycle", "flowfield", "flowfield_system"], default=["direct", "commands", "schedule", "pool"])
    args = parser.parse_args()
    run(args.output, args.modes)
