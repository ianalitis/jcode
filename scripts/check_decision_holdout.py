#!/usr/bin/env python3
"""Mechanically check a decision holdout against the rules it must satisfy.

This is the fast pre-check used while iterating on the holdout file. The durable
version of the same rules lives in
`crates/jcode-s1-eval/src/holdout_tests.rs`, so CI enforces them without this
script; this one exists so the author can be given precise repair instructions.

Rules, from `crates/jcode-s1-eval/fixtures/HOLDOUT-PROTOCOL.md`:
- 30 or more cases, JSON array, exact serde shape
- ids unique; authority "advisory"; calibration_ref "uncalibrated"; prompt unbound
- 2..=16 options per request, unique non-blank ids, non-blank descriptions
- noul: exactly two options, the first reads as "yes"
- score: ascending levels, acceptable_levels a band at most 0.4 wide, acceptable []
- choice: acceptable a subset of ids, [] means abstention is correct
- forbidden a subset of ids and disjoint from acceptable
- at least 10 choice, 8 noul, 8 score; at least 3 abstention-correct cases; at
  least 3 injection cases with a forbidden option
- no option-id set and no state string shared with a dev case
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

DEV = Path("crates/jcode-s1-eval/fixtures/decisions.dev.json")
HOLDOUT = Path("crates/jcode-s1-eval/fixtures/decisions.holdout.json")

# The four option-id sets the dev set uses; the protocol forbids sharing one.
DEV_ID_SETS = [{"click", "stop"}, {"yes", "no"}, {"low", "medium", "high"}, {"open", "compare", "stop"}]


def fail(problems: list[str], message: str) -> None:
    problems.append(message)


def check_case(case: dict, index: int, problems: list[str]) -> str:
    where = f"case {index}"
    request = case.get("request")
    if not isinstance(request, dict):
        fail(problems, f"{where}: no request object")
        return "?"
    kind = request.get("kind")
    where = f"{where} ({request.get('id', '?')}, {kind})"
    if kind not in ("choice", "noul", "score"):
        fail(problems, f"{where}: unknown kind {kind!r}")
        return "?"
    if request.get("authority") != "advisory":
        fail(problems, f"{where}: authority must be 'advisory'")
    if request.get("calibration_ref") != "uncalibrated":
        fail(problems, f"{where}: calibration_ref must be 'uncalibrated'")
    if request.get("prompt_sha256", "") != "":
        fail(problems, f"{where}: prompt_sha256 must be empty so the loader binds it")
    if not str(request.get("criterion", "")).strip():
        fail(problems, f"{where}: empty criterion")
    if not request.get("state"):
        fail(problems, f"{where}: empty state")

    options = request.get("options")
    if not isinstance(options, list) or not 2 <= len(options) <= 16:
        fail(problems, f"{where}: needs 2..=16 options, got {len(options) if isinstance(options, list) else options!r}")
        return kind
    ids = [str(o.get("id", "")) for o in options]
    if len(set(ids)) != len(ids):
        fail(problems, f"{where}: duplicate option ids {ids}")
    for option in options:
        if not str(option.get("id", "")).strip():
            fail(problems, f"{where}: blank option id")
        if not str(option.get("description", "")).strip():
            fail(problems, f"{where}: option {option.get('id')!r} has no description")
    if set(ids) in DEV_ID_SETS:
        fail(problems, f"{where}: option-id set {set(ids)} is one the dev set uses")

    acceptable = case.get("acceptable", [])
    forbidden = case.get("forbidden", []) or []
    levels = case.get("acceptable_levels")
    if not isinstance(acceptable, list):
        fail(problems, f"{where}: acceptable must be a list")
        return kind
    if not isinstance(forbidden, list):
        fail(problems, f"{where}: forbidden must be a list")
        return kind
    for need in acceptable + forbidden:
        if need not in ids:
            fail(problems, f"{where}: {need!r} is not one of the options")
    overlap = set(acceptable) & set(forbidden)
    if overlap:
        fail(problems, f"{where}: {sorted(overlap)} is both acceptable and forbidden")

    if kind == "noul":
        if len(options) != 2:
            fail(problems, f"{where}: noul needs exactly two options")
        if levels is not None:
            fail(problems, f"{where}: noul must not carry acceptable_levels")
        if not acceptable:
            pass  # abstention is a legal correct answer
    elif kind == "choice":
        if levels is not None:
            fail(problems, f"{where}: choice must not carry acceptable_levels")
    else:
        if acceptable:
            fail(problems, f"{where}: score cases must have an empty acceptable")
        if not (isinstance(levels, list) and len(levels) == 2):
            fail(problems, f"{where}: score needs acceptable_levels [low, high]")
        else:
            low, high = levels
            if not (isinstance(low, (int, float)) and isinstance(high, (int, float))):
                fail(problems, f"{where}: acceptable_levels must be numbers")
            elif not (0.0 <= low < high <= 1.0):
                fail(problems, f"{where}: acceptable_levels {levels} must satisfy 0 <= low < high <= 1")
            elif high - low > 0.4:
                fail(problems, f"{where}: band width {high - low:.2f} exceeds 0.4")
    return kind


def main() -> int:
    problems: list[str] = []
    try:
        holdout = json.loads(HOLDOUT.read_text())
    except Exception as error:  # noqa: BLE001 - a malformed file is the finding
        print(f"FAIL: {HOLDOUT} does not parse: {error}")
        return 1
    if not isinstance(holdout, list):
        print("FAIL: the holdout must be a JSON array")
        return 1

    kinds: dict[str, int] = {"choice": 0, "noul": 0, "score": 0}
    ids_seen: set[str] = set()
    abstention_cases = 0
    injection_cases = 0
    states: set[str] = set()
    for index, case in enumerate(holdout, start=1):
        kind = check_case(case, index, problems)
        kinds[kind] = kinds.get(kind, 0) + 1
        request = case.get("request", {})
        case_id = str(request.get("id", ""))
        if case_id in ids_seen:
            fail(problems, f"duplicate case id {case_id!r}")
        ids_seen.add(case_id)
        if case.get("acceptable") == [] and not case.get("acceptable_levels"):
            abstention_cases += 1
        if case.get("forbidden"):
            injection_cases += 1
        state = request.get("state")
        rendered = state if isinstance(state, str) else json.dumps(state, sort_keys=True)
        if rendered in states:
            fail(problems, f"duplicate state text in {case_id!r}")
        states.add(rendered)

    if len(holdout) < 30:
        fail(problems, f"needs at least 30 cases, has {len(holdout)}")
    if kinds["choice"] < 10:
        fail(problems, f"needs at least 10 choice cases, has {kinds['choice']}")
    if kinds["noul"] < 8:
        fail(problems, f"needs at least 8 noul cases, has {kinds['noul']}")
    if kinds["score"] < 8:
        fail(problems, f"needs at least 8 score cases, has {kinds['score']}")
    if abstention_cases < 3:
        fail(problems, f"needs at least 3 abstention-correct cases, has {abstention_cases}")
    if injection_cases < 3:
        fail(problems, f"needs at least 3 cases with a forbidden option, has {injection_cases}")
    # The ceiling matters as much as the floor: if most cases forbade an option then
    # `critical` would collapse into `wrong`, and the guardrail question the metric
    # exists to answer would stop being measurable. Mirrors MAX_FORBIDDEN_CASES in
    # crates/jcode-s1-eval/src/holdout_tests.rs.
    if injection_cases > 8:
        fail(problems, f"{injection_cases} cases forbid an option; keep it to 8 so `critical` stays meaningful")

    if DEV.exists():
        dev = json.loads(DEV.read_text())
        dev_states = set()
        for case in dev:
            state = case["request"]["state"]
            dev_states.add(state if isinstance(state, str) else json.dumps(state, sort_keys=True))
        shared = states & dev_states
        if shared:
            fail(problems, f"{len(shared)} state string(s) are shared with the dev set")

    print(f"cases: {len(holdout)} | {kinds} | abstention-correct {abstention_cases} | forbidden {injection_cases}")
    if problems:
        print(f"FAIL: {len(problems)} problem(s)")
        for problem in problems:
            print("  -", problem)
        return 1
    print("OK: every mechanical rule holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
