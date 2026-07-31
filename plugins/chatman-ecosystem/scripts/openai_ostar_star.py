#!/usr/bin/env python3
"""Bounded OpenAI projection of OStar MuStar and SigmaStar planning contracts.

This adapter imports the real OStar Star classes but deliberately does not call
MuStarAgent.forward() or SigmaStarAggregator.solve(): those paths execute generated
artifacts internally. The adapter uses MuStarPlanner, MuStarExecutor, and
SigmaStarDecomposeSignature as provisional proposers; all actuation remains behind MCP.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any

REQUEST_SCHEMA = "urn:ostar:openai-star-request:v1"
RESULT_SCHEMA = "urn:ostar:openai-star-result:v1"


def _find_ostar_root(explicit: str | None) -> Path:
    candidates: list[Path] = []
    if explicit:
        candidates.append(Path(explicit))
    for name in ("OSTAR_ROOT", "CHATMAN_OSTAR_ROOT"):
        if os.environ.get(name):
            candidates.append(Path(os.environ[name]))
    here = Path(__file__).resolve()
    candidates.append(Path.cwd() / "ostar")
    candidates.extend(parent / "ostar" for parent in here.parents)
    marker = Path("src/ostar/process/mu_star_agent.py")
    for candidate in candidates:
        try:
            root = candidate.expanduser().resolve()
        except OSError:
            continue
        if (root / marker).is_file():
            return root
    raise RuntimeError(
        "cannot resolve OSTAR_ROOT containing src/ostar/process/mu_star_agent.py"
    )


def _configure_openai(model: str) -> None:
    import dspy

    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        raise RuntimeError("OPENAI_API_KEY is required")
    model_id = model if "/" in model else f"openai/{model}"
    kwargs: dict[str, Any] = {
        "model": model_id,
        "api_key": key,
        "temperature": 0.2,
        "max_tokens": 32768,
        "cache": True,
    }
    base = os.environ.get("OPENAI_BASE_URL")
    if base:
        kwargs["api_base"] = base.rstrip("/")
    dspy.configure(lm=dspy.LM(**kwargs))


def _as_bool(value: Any) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in {"true", "yes", "1"}:
            return True
        if normalized in {"false", "no", "0", ""}:
            return False
    if isinstance(value, (int, float)):
        return value != 0
    raise RuntimeError(f"cannot interpret Star boolean: {value!r}")


def _mustar(task_data: dict[str, Any]) -> dict[str, Any]:
    from ostar.process.mu_star_agent import MuStarExecutor, MuStarPlanner
    from ostar.process.mu_star_result import MuStarResult
    from ostar.process.mu_star_task import MuStarTask

    task = MuStarTask(
        domain=str(task_data.get("domain") or "SYSTEM_DESIGN").upper(),
        problem_statement=str(task_data.get("problem_statement") or ""),
        constraints=str(task_data.get("constraints") or ""),
        title=str(task_data.get("title") or "OpenAI Star task"),
        context=dict(task_data.get("context") or {}),
        artifact_type=str(task_data.get("artifact_type") or ""),
    )
    planner = MuStarPlanner(task.domain, store=None)
    plan = planner.forward(task.problem_statement, task.constraints)
    executor = MuStarExecutor()
    executed = executor.forward(
        problem_statement=task.problem_statement,
        build_order=plan.build_order,
        powl_model=plan.powl_model,
        sequence_diagram=plan.sequence_diagram,
    )
    result = MuStarResult(
        title=task.title,
        domain=task.domain,
        build_order=plan.build_order,
        artifact=executed.artifact,
        artifact_type=executed.artifact_type,
        operator_notation=executed.operator_notation,
        build_order_adhered=_as_bool(executed.build_order_adhered),
        implementation_complete=_as_bool(executed.implementation_complete),
    )
    value = result.to_dict()
    value["powl_model"] = plan.powl_model
    value["sequence_diagram"] = plan.sequence_diagram
    return value


def _sigma_tasks(prompt: str, constraints: str, max_tasks: int) -> list[dict[str, Any]]:
    import dspy
    from ostar.process.sigma_star import SigmaStarDecomposeSignature

    prediction = dspy.Predict(SigmaStarDecomposeSignature)(
        objective=prompt,
        constraints=constraints,
    )
    raw = json.loads(prediction.task_list_json)
    if not isinstance(raw, list):
        raise RuntimeError("SigmaStar decomposition was not a JSON list")
    tasks = []
    for value in raw[:max_tasks]:
        if not isinstance(value, dict):
            continue
        tasks.append(
            {
                "domain": value.get("domain", "SYSTEM_DESIGN"),
                "title": value.get("title", "Untitled SigmaStar task"),
                "problem_statement": value.get("problem_statement", ""),
                "constraints": value.get("constraints", constraints),
                "artifact_type": value.get("artifact_type", ""),
            }
        )
    if not tasks:
        raise RuntimeError("SigmaStar decomposition produced no admissible tasks")
    return tasks


def main() -> int:
    request = json.load(sys.stdin)
    if not isinstance(request, dict) or request.get("schema") != REQUEST_SCHEMA:
        raise RuntimeError("request schema mismatch")
    root = _find_ostar_root(os.environ.get("OSTAR_ROOT"))
    sys.path.insert(0, str(root / "src"))
    model = str(request.get("model") or "")
    _configure_openai(model)
    prompt = str(request.get("prompt") or "")
    target = str(request.get("target") or "")
    constraints = (
        f"Target repository: {target}. All actuation must occur through MCP; "
        "this Star output is provisional."
    )
    mode = str(request.get("mode") or "mustar")
    domain = str(request.get("domain") or "SYSTEM_DESIGN").upper()
    max_tasks = int(request.get("max_tasks") or 8)
    if mode == "sigma-star":
        tasks = _sigma_tasks(prompt, constraints, max_tasks)
    elif mode == "mustar":
        tasks = [
            {
                "domain": domain,
                "title": f"{target} MuStar task",
                "problem_statement": prompt,
                "constraints": constraints,
            }
        ]
    else:
        raise RuntimeError(f"unsupported Star mode: {mode}")
    results = [_mustar(task) for task in tasks]
    output = {
        "schema": RESULT_SCHEMA,
        "mode": mode,
        "target": target,
        "provisional": True,
        "authority": "proposer",
        "internal_actuation": False,
        "ostar_root": str(root),
        "star_classes": {
            "planner": "ostar.process.mu_star_agent.MuStarPlanner",
            "executor": "ostar.process.mu_star_agent.MuStarExecutor",
            "decomposer": (
                "ostar.process.sigma_star.SigmaStarDecomposeSignature"
                if mode == "sigma-star"
                else None
            ),
        },
        "results": results,
    }
    print(json.dumps(output, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(
            json.dumps({"code": "OSTAR_STAR_FAILED", "message": str(error)}),
            file=sys.stderr,
        )
        raise SystemExit(1) from None
