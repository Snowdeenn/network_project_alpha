"""Control the real Rust ECS through its headless JSON-lines example."""
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


class Simulation:
    def __init__(self, build=True):
        if build:
            subprocess.run(["cargo", "build", "-p", "server", "--example", "simulation_bridge", "--locked"], cwd=ROOT, check=True)
        metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=ROOT))
        executable = Path(metadata["target_directory"]) / "debug" / "examples" / ("simulation_bridge.exe" if os.name == "nt" else "simulation_bridge")
        self.process = subprocess.Popen([str(executable)], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding="utf-8")

    def command(self, op, **kwargs):
        self.process.stdin.write(json.dumps({"op": op, **kwargs}, allow_nan=False) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError(f"Rust simulation stopped (exit={self.process.poll()})")
        result = json.loads(line)
        if not result["ok"]:
            raise RuntimeError(result["error"])
        return result["data"]

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.process.stdout.close()


def fireball_scenario(build=True):
    with Simulation(build) as game:
        game.command("player", id=1, **{"class": "Warrior"}, position=[200, 200])
        game.command("player", id=2, **{"class": "Tank"}, position=[200, 500])
        game.command("enemy", id=10, position=[300, 200], hp=200)
        game.command("gold", id=1, amount=10)
        game.command("equip", id=1, spell="fireball", slot=0)
        game.command("input", id=1, aim=[1, 0], slot=0)
        game.command("step", ticks=1)
        first = game.command("snapshot")
        assert first["entities"]["1"]["gold"] == 7, first
        assert first["projectiles"] == 1, first
        game.command("input", id=1, aim=[1, 0], slot=0)
        game.command("step", ticks=1)
        rejected = game.command("snapshot")
        assert rejected["entities"]["1"]["gold"] == 7, rejected
        assert any("SpellCastError" in event["kind"] for event in rejected["events"]), rejected
        game.command("step", ticks=100)
        final = game.command("snapshot")
        assert final["entities"]["10"]["hp"] == 140, final  # 40 impact + 4 burn ticks of 5
        assert final["entities"]["2"]["hp"] == final["entities"]["2"]["max_hp"], final
        assert final["entities"]["1"]["cooldowns"][0] == 0, final
        assert final["projectiles"] == 0, final
        return {"scenario": "fireball", "ticks": final["tick"], "enemy_hp": final["entities"]["10"]["hp"], "gold": final["entities"]["1"]["gold"]}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()
    print(json.dumps(fireball_scenario(not args.no_build), ensure_ascii=False, indent=2))
