"""Run real network players against the production server, record workload telemetry."""
import argparse
from collections import Counter
import json
import math
import os
from pathlib import Path
import queue
import subprocess
import threading
import time

from simulation import ROOT
from process_metrics import ProcessMetrics, distribution, summarize_ticks


def read_lines(pipe, output):
    try:
        for line in pipe:
            output.put(line)
    finally:
        output.put(None)


def stop(process):
    if process and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def run(args):
    if not args.no_build:
        subprocess.run(["cargo", "build", "-p", "server", "--bin", "server", "--example", "network_bots", "--locked"] + (["--release"] if args.release else []) + (["--features", "allocation-metrics"] if args.allocation_metrics else []), cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=ROOT))
    suffix = ".exe" if os.name == "nt" else ""
    target = Path(metadata["target_directory"]) / ("release" if args.release else "debug")
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    server = bots = None
    summary = {"players": args.players, "profile": args.profile, "input_hz": args.hz, "cast_hz": args.cast_hz, "requested_seconds": args.seconds, "snapshots": 0, "payload_bytes": 0, "input_messages": 0, "cast_requests": 0}
    events = Counter()
    snapshots = {}
    entity_ids = {}
    slots = {}
    joined = set()
    ready = False
    starts = set()
    last_shop = {}
    last_respawn = {}
    last_cast = {}
    pending_actions = []
    monitors = {}
    process_samples = {}
    cycle_timings = []
    last_sample = 0
    intervals = []
    arrivals = {}
    incoming = queue.Queue()
    started = time.monotonic()
    summary["started_unix_ms"] = time.time_ns() // 1_000_000
    summary["build_profile"] = "release" if args.release else "debug"
    summary["sample_interval_seconds"] = args.sample_interval
    summary["logical_cpu_count"] = os.cpu_count()
    error = None
    with (output / "server.log").open("w", encoding="utf-8") as logs, (output / "telemetry.jsonl").open("w", encoding="utf-8") as trace, \
         (output / "resources.jsonl").open("w", encoding="utf-8") as resource_log, (output / "events.jsonl").open("w", encoding="utf-8") as event_log:
        def log_event(kind, **data):
            event_log.write(json.dumps({"unix_time_ms": time.time_ns() // 1_000_000, "elapsed": time.monotonic() - started, "kind": kind, **data}) + "\n")
            event_log.flush()
        try:
            if not args.external_server:
                server_env = os.environ.copy()
                server_env["NETWORK_ALPHA_METRICS"] = str(output / "server-ticks.jsonl")
                server_env["RUST_LOG"] = args.log_level
                server = subprocess.Popen([str(target / ("server" + suffix))], cwd=ROOT, stdout=logs, stderr=subprocess.STDOUT, env=server_env)
                log_event("server_started", pid=server.pid)
                time.sleep(0.5)
                if server.poll() is not None:
                    raise RuntimeError("Server failed to start; see server.log (port may be occupied)")
            bots = subprocess.Popen([str(target / "examples" / ("network_bots" + suffix)), str(args.players), args.address], cwd=ROOT,
                                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=logs, text=True, encoding="utf-8")
            reader = threading.Thread(target=read_lines, args=(bots.stdout, incoming), daemon=True)
            reader.start()
            pids = {"python": os.getpid(), "bots": bots.pid}
            if server:
                pids["server"] = server.pid
            elif args.server_pid:
                pids["server"] = args.server_pid
            for name, pid in pids.items():
                monitors[name] = ProcessMetrics(pid)
                process_samples[name] = []
            log_event("workload_started", parameters=vars(args), pids=pids)
            tick = 0
            deadline = time.monotonic() + args.seconds
            while time.monotonic() < deadline:
                now = time.monotonic()
                cycle_start = now
                if now - last_sample >= args.sample_interval:
                    for name, monitor in monitors.items():
                        try:
                            sample = monitor.sample()
                            process_samples[name].append(sample)
                            resource_log.write(json.dumps({"unix_time_ms": time.time_ns() // 1_000_000, "elapsed": now - started, "process": name, **sample}) + "\n")
                        except (OSError, RuntimeError) as exc:
                            log_event("process_sample_error", process=name, error=str(exc))
                    resource_log.flush()
                    last_sample = now
                if server and server.poll() is not None:
                    raise RuntimeError("Server stopped during workload; see server.log")
                actions = []
                if not ready and len(joined) == args.players:
                    for bot in range(args.players):
                        actions.extend([{"bot": bot, "channel": "lobby", "data": {"ClassSelected": {"class": ["Warrior", "Assassin", "Mage", "Tank"][bot]}}},
                                        {"bot": bot, "channel": "lobby", "data": "ToggleReady"}])
                    ready = True
                for bot in starts:
                    state = snapshots.get(bot, {})
                    entities = state.get("entities", [])
                    own = next((e for e in entities if e["entity_id"] == entity_ids.get(bot)), None)
                    aim = [1.0, 0.0]
                    movement = [0.0, 0.0]
                    if own:
                        enemies = [e for e in entities if e["entity_kind"] == "Enemy" and e["health"] > 0]
                        if enemies:
                            enemy = min(enemies, key=lambda e: math.dist(e["position"], own["position"]))
                            dx, dy = [enemy["position"][i] - own["position"][i] for i in (0, 1)]
                            distance = math.hypot(dx, dy)
                            if distance:
                                aim = [dx / distance, dy / distance]
                                if args.profile == "combat":
                                    movement = aim if distance > 65 else [0.0, 0.0]
                        if args.profile == "movement":
                            angle = now - started + bot
                            movement = [math.cos(angle), math.sin(angle)]
                    cast = None
                    if args.profile == "combat" and bot in slots and now - last_cast.get(bot, 0) >= 1 / args.cast_hz:
                        cast = slots[bot]
                        last_cast[bot] = now
                    actions.append({"bot": bot, "channel": "input", "data": {"move_dir": movement, "aim_dir": aim, "tick_id": tick,
                                   "spell": cast, "dash": False, "attack": args.profile == "combat"}})
                    phase = state.get("wave_info", {}).get("wave_state")
                    if args.profile == "combat" and isinstance(phase, dict) and "BetweenWave" in phase and now - last_shop.get(bot, 0) > 2:
                        actions.append({"bot": bot, "channel": "shop", "data": {"kind": "Open", "slot": 0}})
                        last_shop[bot] = now
                    if own and own["health"] == 0 and now - last_respawn.get(bot, 0) > 2:
                        actions.append({"bot": bot, "channel": "event", "data": {"kind": {"RequestRespawn": {"client_id": ids[bot], "option": "UseSharedLife"}}}})
                        last_respawn[bot] = now
                # Shop purchases are queued after receiving the authoritative offer.
                actions.extend(pending_actions)
                pending_actions = []
                summary["input_messages"] += sum(action["channel"] == "input" for action in actions)
                summary["cast_requests"] += sum(action["channel"] == "input" and action["data"]["spell"] is not None for action in actions)
                decision_ms = (time.monotonic() - cycle_start) * 1000
                request_start = time.monotonic()
                bots.stdin.write(json.dumps({"actions": actions, "wait_ms": max(1, round(1000 / args.hz))}) + "\n")
                bots.stdin.flush()
                try:
                    line = incoming.get(timeout=5)
                except queue.Empty:
                    raise RuntimeError("No response from network adapter for five seconds")
                if line is None:
                    raise RuntimeError("Network adapter stopped; see server.log")
                response = json.loads(line)
                exchange_ms = (time.monotonic() - request_start) * 1000
                received_at = time.monotonic() - started
                ids = [b["client_id"] for b in response["bots"]]
                trace.write(json.dumps({"elapsed": received_at, "sent": actions, **response}) + "\n")
                summary["payload_bytes"] += response["received_payload_bytes"]
                for message in response["messages"]:
                    bot, channel, data = message["bot"], message["channel"], message["data"]
                    if channel == "lobby":
                        if "SessionJoined" in data:
                            joined.add(bot)
                        if "GameStarting" in data:
                            starts.add(bot)
                    elif channel == "snapshot":
                        snapshots[bot] = data
                        summary["snapshots"] += 1
                        if bot in arrivals:
                            intervals.append(received_at - arrivals[bot])
                        arrivals[bot] = received_at
                    else:
                        kind = data["kind"]
                        name = next(iter(kind)) if isinstance(kind, dict) else kind
                        log_event("game_event", bot=bot, event=kind)
                        events[name] += 1
                        if name == "PlayerSpawn" and kind[name]["client_id"] == ids[bot]:
                            entity_ids[bot] = kind[name]["entity_id"]
                        if name == "SpellAcquired":
                            slots[bot] = kind[name]["slot"]
                        if name == "ShopOpened":
                            gold = snapshots.get(bot, {}).get("player_info") or {}
                            for slot, spell in enumerate(kind[name]["inventory"]):
                                if spell and spell["purchase_cost"]["gold"] <= gold.get("gold", 0):
                                    pending_actions.append({"bot": bot, "channel": "shop", "data": {"kind": "Buy", "slot": slot}})
                                    break
                if not ready and received_at > 10:
                    raise RuntimeError("Not all bots joined within ten seconds")
                cycle_timings.append({"decision_ms": decision_ms, "adapter_exchange_ms": exchange_ms,
                                      "cycle_ms": (time.monotonic() - cycle_start) * 1000})
                trace.write(json.dumps({"elapsed": time.monotonic() - started, "timings": cycle_timings[-1]}) + "\n")
                trace.flush()
                tick += 1
            if len(starts) != args.players or not snapshots:
                raise RuntimeError("Workload ended without all players starting and receiving snapshots")
        except Exception as exc:
            error = str(exc)
            log_event("workload_error", error=error)
        finally:
            log_event("workload_stopping", server_exit_code=server.poll() if server else None, bot_exit_code=bots.poll() if bots else None)
            for monitor in monitors.values():
                monitor.close()
            stop(bots)
            if bots:
                bots.stdin.close()
                bots.stdout.close()
            stop(server)
    summary.update({"elapsed_seconds": time.monotonic() - started, "events": dict(events), "joined": len(joined), "started": len(starts), "error": error,
                    "max_snapshot_delivery_gap_seconds": max(intervals, default=None), "final_snapshots": snapshots})
    summary["server_ticks"] = summarize_ticks(output / "server-ticks.jsonl")
    summary["processes"] = {name: {
        "samples": len(samples),
        "cpu_percent_one_core": distribution([s["cpu_percent_one_core"] for s in samples if s["cpu_percent_one_core"] is not None]),
        "rss_bytes": distribution([s["rss_bytes"] for s in samples]),
        "private_bytes": distribution([s["private_bytes"] for s in samples if s["private_bytes"] is not None]),
        "cpu_seconds_during_sampling": samples[-1]["cpu_seconds"] - samples[0]["cpu_seconds"] if len(samples) > 1 else None,
        "io_read_bytes_during_sampling": samples[-1]["io_read_bytes"] - samples[0]["io_read_bytes"] if len(samples) > 1 else None,
        "io_write_bytes_during_sampling": samples[-1]["io_write_bytes"] - samples[0]["io_write_bytes"] if len(samples) > 1 else None,
    } for name, samples in process_samples.items()}
    summary["driver_timings_ms"] = {field: distribution([cycle[field] for cycle in cycle_timings]) for field in ("decision_ms", "adapter_exchange_ms", "cycle_ms")}
    (output / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"Workload {'FAILED' if error else 'completed'}: {output}")
    print(f"Players started: {len(starts)}, snapshots: {summary['snapshots']}, payload bytes: {summary['payload_bytes']}")
    if error:
        raise RuntimeError(error)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--players", type=int, choices=range(1, 5), default=2)
    parser.add_argument("--seconds", type=float, default=60)
    parser.add_argument("--hz", type=int, choices=range(1, 201), default=20)
    parser.add_argument("--cast-hz", type=float, default=1, help="spell attempts per second per equipped bot")
    parser.add_argument("--profile", choices=["idle", "movement", "combat"], default="combat")
    parser.add_argument("--output", default="workload-results/" + time.strftime("%Y%m%d-%H%M%S"))
    parser.add_argument("--address", default="127.0.0.1:7777")
    parser.add_argument("--external-server", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--release", action="store_true", help="build/run optimized binaries for performance comparisons")
    parser.add_argument("--allocation-metrics", action="store_true", help="build the server with live heap and per-phase allocation accounting")
    parser.add_argument("--sample-interval", type=float, default=.5)
    parser.add_argument("--server-pid", type=int, help="measure an external local server process")
    parser.add_argument("--log-level", default="info", help="RUST_LOG filter for the launched server")
    args = parser.parse_args()
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        parser.error("--seconds must be positive and finite")
    if not math.isfinite(args.cast_hz) or args.cast_hz <= 0:
        parser.error("--cast-hz must be positive and finite")
    if not math.isfinite(args.sample_interval) or args.sample_interval <= 0:
        parser.error("--sample-interval must be positive and finite")
    if args.server_pid and not args.external_server:
        parser.error("--server-pid requires --external-server")
    if not args.external_server and args.address != "127.0.0.1:7777":
        parser.error("custom --address requires --external-server")
    run(args)
