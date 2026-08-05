#!/usr/bin/env python3
import unittest

from ci_router import route


class RouterTests(unittest.TestCase):
    def test_docs_only(self):
        self.assertEqual(route(["docs/guide.md"])["docs"], ["docs/guide.md"])
        self.assertFalse(route(["docs/guide.md"])["rust"])

    def test_source_and_test_route_to_rust(self):
        routed = route(["crates/ferroplan/src/lib.rs", "crates/ferroplan/tests/api.rs"])
        self.assertEqual(len(routed["rust"]), 2)

    def test_browser_owns_wasm_and_bevy(self):
        routed = route(["crates/ferroplan-wasm/src/lib.rs", "crates/ferroplan-bevy/src/main.rs"])
        self.assertEqual(len(routed["browser"]), 2)

    def test_workflow_only_is_admission_not_product(self):
        routed = route([".github/workflows/fortune5-admission.yml"])
        self.assertEqual(routed["admission"], [".github/workflows/fortune5-admission.yml"])
        self.assertFalse(routed["rust"])
        self.assertFalse(routed["browser"])

    def test_fast_workflow_is_fast_only(self):
        routed = route([".github/workflows/errc-fast.yml"])
        self.assertEqual(routed["fast_only"], [".github/workflows/errc-fast.yml"])

    def test_multi_lane_and_duplicate_paths(self):
        routed = route(["Cargo.lock", "Cargo.lock", "crates/ferroplan-py/src/lib.rs"])
        self.assertEqual(routed["rust"].count("Cargo.lock"), 1)
        self.assertIn("Cargo.lock", routed["supply_chain"])
        self.assertIn("crates/ferroplan-py/src/lib.rs", routed["python"])

    def test_exclusion_precedence(self):
        routed = route(["README.md"])
        self.assertEqual(routed["docs"], ["README.md"])
        self.assertFalse(routed["admission"])


if __name__ == "__main__":
    unittest.main()
