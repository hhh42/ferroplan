from __future__ import annotations

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "verify_openai_luna.py"
SPEC = importlib.util.spec_from_file_location("verify_openai_luna", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def write_junit(
    path: Path,
    failing: str | None = None,
    nested_unittest_module: str | None = None,
) -> None:
    cases = []
    for _name, module in MODULE.CATEGORIES:
        failure = "<failure message='boom'/>" if failing == module else ""
        classname = (
            f"{module}.StarRuntimeTests"
            if nested_unittest_module == module
            else f"tests.{module}"
        )
        cases.append(
            f"<testcase classname='{classname}' name='witness'>{failure}</testcase>"
        )
    path.write_text(
        "<testsuites><testsuite>" + "".join(cases) + "</testsuite></testsuites>",
        encoding="utf-8",
    )


def test_verifier_report_alive_and_sealed(tmp_path: Path) -> None:
    junit = tmp_path / "junit.xml"
    write_junit(junit, nested_unittest_module="test_openai_luna")
    report = MODULE.build_report(
        junit, ["ruff", "compileall", "shell-syntax"], "3.11"
    )
    assert report["standing"] == "ALIVE"
    assert report["summary"]["alive"] == len(MODULE.CATEGORIES)
    assert report["report_sha256"].startswith("sha256:")
    legacy = next(
        item for item in report["categories"] if item["name"] == "legacy_regression"
    )
    assert legacy["tests"] == 1


def test_verifier_report_propagates_failure(tmp_path: Path) -> None:
    junit = tmp_path / "junit.xml"
    write_junit(junit, "test_openai_luna_chaos")
    report = MODULE.build_report(junit, [], "3.11")
    assert report["standing"] == "BUILD_BROKEN"
    chaos = next(item for item in report["categories"] if item["name"] == "chaos")
    assert chaos["failures"] == 1
