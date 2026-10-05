"""Integration scenarios against a previously built Rust simulation bridge."""
import unittest
from simulation import Simulation, fireball_scenario


class Scenarios(unittest.TestCase):
    def test_fireball_impact_burn_and_cooldown(self):
        self.assertEqual(fireball_scenario(build=False)["enemy_hp"], 140)

    def test_movement_leaves_other_player_in_place(self):
        with Simulation(build=False) as game:
            for player in (1, 2):
                game.command("player", id=player, **{"class": "Warrior"}, position=[200, 200])
            game.command("input", id=1, aim=[1, 0], movement=[1, 0])
            game.command("step", ticks=20)
            state = game.command("snapshot")
            self.assertGreater(state["entities"]["1"]["position"][0], 200)
            self.assertEqual(state["entities"]["2"]["position"], [200, 200])
            self.assertEqual(state["entities"]["1"]["position"][1], 200)

    def test_cast_without_gold_is_rejected_without_projectile(self):
        with Simulation(build=False) as game:
            game.command("player", id=1, **{"class": "Mage"}, position=[200, 200])
            game.command("equip", id=1, spell="fireball", slot=0)
            game.command("input", id=1, aim=[1, 0], slot=0)
            game.command("step", ticks=1)
            state = game.command("snapshot")
            self.assertEqual(state["entities"]["1"]["gold"], 0)
            self.assertEqual(state["entities"]["1"]["cooldowns"][0], 0)
            self.assertEqual(state["projectiles"], 0)
            self.assertTrue(any(event["client_id"] == 1 and event["kind"].get("SpellCastError", {}).get("reason") == "NotEnoughtGold" for event in state["events"]))


if __name__ == "__main__":
    unittest.main()
