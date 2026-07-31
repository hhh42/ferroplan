from __future__ import annotations

import sys
import types
from pathlib import Path
from types import SimpleNamespace

import pytest

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))
import openai_ostar_star as star


@pytest.mark.parametrize(("value", "expected"), [(True, True), (False, False), ("yes", True), ("0", False), (1, True), (0, False)])
def test_star_boolean_contract(value, expected) -> None:
    assert star._as_bool(value) is expected


def test_star_boolean_refuses_ambiguous_value() -> None:
    with pytest.raises(RuntimeError, match="cannot interpret"):
        star._as_bool(object())


def test_find_ostar_root_requires_marker(tmp_path: Path) -> None:
    root = tmp_path / "ostar"
    marker = root / "src/ostar/process/mu_star_agent.py"
    marker.parent.mkdir(parents=True)
    marker.write_text("# marker\n", encoding="utf-8")
    assert star._find_ostar_root(str(root)) == root.resolve()


def test_mustar_uses_planner_executor_without_agent_forward(monkeypatch) -> None:
    calls = []

    class Task:
        def __init__(self, **kwargs):
            self.__dict__.update(kwargs)

    class Planner:
        def __init__(self, domain, store=None):
            calls.append(("planner_init", domain, store))

        def forward(self, problem, constraints):
            calls.append(("planner_forward", problem, constraints))
            return SimpleNamespace(build_order="order", powl_model="POWL", sequence_diagram="diagram")

    class Executor:
        def forward(self, **kwargs):
            calls.append(("executor_forward", kwargs))
            return SimpleNamespace(
                artifact="candidate",
                artifact_type="spec",
                operator_notation="compile",
                build_order_adhered="yes",
                implementation_complete=True,
            )

    class Result:
        def __init__(self, **kwargs):
            self.kwargs = kwargs

        def to_dict(self):
            return dict(self.kwargs)

    modules = {
        "ostar": types.ModuleType("ostar"),
        "ostar.process": types.ModuleType("ostar.process"),
        "ostar.process.mu_star_agent": types.ModuleType("ostar.process.mu_star_agent"),
        "ostar.process.mu_star_result": types.ModuleType("ostar.process.mu_star_result"),
        "ostar.process.mu_star_task": types.ModuleType("ostar.process.mu_star_task"),
    }
    modules["ostar.process.mu_star_agent"].MuStarPlanner = Planner
    modules["ostar.process.mu_star_agent"].MuStarExecutor = Executor
    modules["ostar.process.mu_star_result"].MuStarResult = Result
    modules["ostar.process.mu_star_task"].MuStarTask = Task
    for name, module in modules.items():
        monkeypatch.setitem(sys.modules, name, module)

    value = star._mustar(
        {
            "domain": "SYSTEM_DESIGN",
            "problem_statement": "build it",
            "constraints": "no actuation",
            "title": "task",
        }
    )
    assert value["artifact"] == "candidate"
    assert value["powl_model"] == "POWL"
    assert value["sequence_diagram"] == "diagram"
    assert [call[0] for call in calls] == ["planner_init", "planner_forward", "executor_forward"]


def test_sigma_decomposition_is_bounded(monkeypatch) -> None:
    dspy = types.ModuleType("dspy")

    class Predict:
        def __init__(self, signature):
            self.signature = signature

        def __call__(self, **kwargs):
            assert kwargs["objective"] == "objective"
            return SimpleNamespace(
                task_list_json='[{"title":"a"},{"title":"b"},{"title":"c"}]'
            )

    dspy.Predict = Predict
    sigma = types.ModuleType("ostar.process.sigma_star")
    sigma.SigmaStarDecomposeSignature = object
    monkeypatch.setitem(sys.modules, "dspy", dspy)
    monkeypatch.setitem(sys.modules, "ostar.process.sigma_star", sigma)
    tasks = star._sigma_tasks("objective", "bounded", 2)
    assert [task["title"] for task in tasks] == ["a", "b"]


def test_configure_openai_pins_requested_model(monkeypatch) -> None:
    configured = []
    dspy = types.ModuleType("dspy")

    class LM:
        def __init__(self, **kwargs):
            self.kwargs = kwargs

    dspy.LM = LM
    dspy.configure = lambda **kwargs: configured.append(kwargs)
    monkeypatch.setitem(sys.modules, "dspy", dspy)
    monkeypatch.setenv("OPENAI_API_KEY", "test-key")
    star._configure_openai("gpt-5.6-luna")
    assert configured[0]["lm"].kwargs["model"] == "openai/gpt-5.6-luna"
    assert configured[0]["lm"].kwargs["api_key"] == "test-key"
