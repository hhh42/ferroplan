#!/usr/bin/env python3
"""Unit tests for standings.py's archive parsers and classify() budget
stamp (0.23 Phase 3 desk work; run: python3 benchmarks/test_standings.py).

The makespan expectations are hand-computed from the vendored
IPC5-results.tgz — three planners, three formats, on the record here so a
regex regression cannot silently re-score a board:

  sgplan.ipc04/storage/Time/p01.soln   (empty `; MakeSpan` header — the
      reason the parser NEVER trusts headers; bracket glued to the paren):
        0.010+1.000, 1.020+2.000, 3.030+2.000       -> 5.030
  mips-xxl/storage/Time/p01.soln       (header present and agreeing):
        0.00+1.00, 1.00+2.00, 1.00+2.00             -> 3.00
  mips-xxl/TPP/MetricTime/p01.soln     (large values, spaced brackets):
        ... 2717.02+17.00                           -> 2734.02
  yochanps/storage/Time/p01.soln       (lowercase actions; its own
      header says 3.02 — the computed 3.03 is one eps slot off the
      header, which is exactly why one instrument is used for all):
        0.01+1.0, 1.02+2.0, 1.03+2.0                -> 3.03
"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import standings  # noqa: E402


class ArchiveMakespans(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ms = standings.archive_makespans()

    def test_archive_present(self):
        self.assertTrue(self.ms, "IPC5-results.tgz missing or empty")

    def test_sgplan_empty_header_computed_from_steps(self):
        self.assertAlmostEqual(
            self.ms[("storage", "Time", 1)]["sgplan.ipc04"], 5.030, places=3)

    def test_mips_xxl_time_and_metric_time(self):
        self.assertAlmostEqual(
            self.ms[("storage", "Time", 1)]["mips-xxl"], 3.00, places=3)
        self.assertAlmostEqual(
            self.ms[("TPP", "MetricTime", 1)]["mips-xxl"], 2734.02, places=2)

    def test_yochanps_lowercase_steps(self):
        self.assertAlmostEqual(
            self.ms[("storage", "Time", 1)]["yochanps"], 3.03, places=3)

    def test_propositional_members_not_parsed(self):
        # The makespan pass reads Time*/MetricTime* members only; a
        # propositional key in this dict would mean it paid for (and could
        # mis-join against) tracks that have no makespan currency.
        self.assertNotIn(("storage", "Propositional", 1), self.ms)

    def test_arch_track_maps_the_two_reentry_variants(self):
        self.assertEqual(standings.arch_track("storage-time"),
                         ("storage", "Time"))
        self.assertEqual(standings.arch_track("trucks-time-strips"),
                         ("trucks", "Time/Strips-Time"))
        self.assertEqual(standings.arch_track("tpp-metric-time"),
                         ("TPP", "MetricTime"))


class MakespanQuality(unittest.TestCase):
    def test_scores_only_rows_carrying_makespan(self):
        arch = {("storage", "Time", 1): {"a": 10.0, "b": 12.0}}
        pre_022 = [{"variant": "storage-time", "instance": 1, "solved": True,
                    "val": True}]
        self.assertIsNone(standings.makespan_quality(pre_022, arch),
                          "a raw without the 0.22 makespan column must not "
                          "acquire a quality number")
        scored = [{"variant": "storage-time", "instance": 1, "solved": True,
                   "val": True, "makespan": 20.0}]
        q = standings.makespan_quality(scored, arch)
        self.assertIn("0W/0T/1L", q)
        self.assertIn("0.50", q)  # best-of-field 10 / ours 20

    def test_eps_bookkeeping_is_a_tie_not_a_loss(self):
        arch = {("storage", "Time", 1): {"a": 10.0}}
        rows = [{"variant": "storage-time", "instance": 1, "solved": True,
                 "val": True, "makespan": 10.01}]
        self.assertIn("0W/1T/0L", standings.makespan_quality(rows, arch))


class ClassifyBudgetStamp(unittest.TestCase):
    """The tier-move mechanism: a row's own budget wins over the registry."""

    def test_stamped_row_ignores_registry_budget(self):
        row = {"solved": False, "time": 29.5, "budget": 30, "notes": None}
        # Registry says 60 (post-flip): the 30 s-stamped wall-exit must
        # still read timeout, not early-exit.
        self.assertEqual(standings.classify(row, 60), "timeout")

    def test_unstamped_row_uses_registry_budget(self):
        row = {"solved": False, "time": 29.5, "notes": None}
        self.assertEqual(standings.classify(row, 30), "timeout")
        self.assertEqual(standings.classify(row, 60), "early-exit")

    def test_sixty_second_stamp_under_lagging_registry(self):
        # The deferral window itself: registry still 30, raw already 60.
        row = {"solved": False, "time": 58.9, "budget": 60, "notes": None}
        self.assertEqual(standings.classify(row, 30), "timeout")


class PrefPoints(unittest.TestCase):
    """The preference boards' points as NUMBERS (0.29): the front page prints
    them beside the rows, ipc-standings.md prints them as a sentence, and
    both must come from one sum."""
    ARCH = {("storage", "QualitativePreferences", 1): {"sgplan": 10.0},
            ("storage", "QualitativePreferences", 2): {"sgplan": 10.0},
            ("storage", "QualitativePreferences", 3): {"sgplan": 10.0}}

    def rows(self):
        v = "storage-preferences-qualitative"
        return [
            {"variant": v, "instance": 1, "solved": True, "val": True,
             "metric": 5.0},     # we win: ours 1.0, theirs 0.5
            {"variant": v, "instance": 2, "solved": True, "val": True,
             "metric": None},    # solved unpriced: ours 0, theirs 1.0
            {"variant": v, "instance": 3, "solved": False, "val": True},
        ]

    def test_points_and_sentence_agree(self):
        p = standings.pref_points(self.rows(), self.ARCH)
        self.assertEqual(p["n"], 3)
        self.assertAlmostEqual(p["ours"], 1.0)
        self.assertAlmostEqual(p["theirs"], 2.5)
        self.assertEqual((p["w"], p["t"], p["l"], p["unpriced"]), (1, 0, 0, 1))
        self.assertEqual(
            standings.pref_quality(self.rows(), self.ARCH),
            "IPC score vs SGPlan5: 1.0 / 2.5 over 3 cells (1W/0T/0L; "
            "1 solved unpriced)")

    def test_nothing_scored_is_none(self):
        self.assertIsNone(standings.pref_points(self.rows(), {}))


class BoardPlacement(unittest.TestCase):
    """`_placement` (STANDINGS.md) and `_placement_brief` (the README) render
    ONE rank: the brief form drops the words, never the number."""
    COHORT = {"entrants": [["LAMA", 281, 300], ["FF", 225, 270]],
              "field_size": 10}

    def test_both_forms_share_the_rank(self):
        self.assertEqual(standings._placement(self.COHORT, 298, 300),
                         "~1st of 11 by rate (leader LAMA 281/300)")
        self.assertEqual(standings._placement_brief(self.COHORT, 298, 300),
                         "~1st of 11 (LAMA 281/300)")
        self.assertEqual(standings._placement_brief(self.COHORT, 240, 300),
                         "~3rd of 11 (LAMA 281/300)")
        self.assertIsNone(standings._placement_brief(self.COHORT, 0, 0))

    def test_front_rows_group_split_and_dash(self):
        """A split board renders one slot per competition, the first named;
        a board with no cohort takes its slot as a dash; an empty group is
        not a row; order is SWEEPS order."""
        rows_ss = [{"ipc": "ipc-2008", "variant": "x", "instance": 1,
                    "solved": True, "val": True},
                   {"ipc": "ipc-2011", "variant": "x", "instance": 1,
                    "solved": False, "val": True}]
        rows_18 = [{"ipc": "ipc-2018", "variant": "x", "instance": 1,
                    "solved": True, "val": True}]
        data = {label: None for label, _c, _b in standings.SWEEPS.values()}
        live = [("seq-sat", 1, 2, 50.0, rows_ss),
                ("2018 seq-sat", 1, 1, 100.0, rows_18)]
        field = {"seq-sat": {"splits": {
            "ipc-2008": {"entrants": [["LAMA", 1, 2]], "field_size": 3},
            "ipc-2011": {"entrants": [["LAMA-2011", 2, 2]]}}}}
        out = standings._front_rows(data, live, field, {})
        self.assertEqual(out, [(
            "classical satisficing",
            "seq-sat 2008 1/1 · 2011 0/1 · 2018 1/1",
            "~1st of 4 (LAMA 1/2) · 2nd of 2 (LAMA-2011 2/2) · —")])


if __name__ == "__main__":
    unittest.main(verbosity=2)
