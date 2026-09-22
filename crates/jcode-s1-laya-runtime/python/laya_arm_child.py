#!/usr/bin/env python3
"""The W3 local decision arm's child process.

One short-lived process per batch. It reads one `DecisionRequest` (the W1
contract's serde_json form) per line on stdin, writes one line per request on
stdout, and ends with a single summary line. It holds no policy: it does not
threshold, rank against a baseline, or decide to abstain. An error is reported as
an error and what to do with it is the caller's decision, per the contract's rule
that a failure must be visible rather than default-allow.

Line kinds, all JSON objects with a `kind` field:

  {"kind":"ready","device":"mps","model":"...","load_ms":29300}
  {"kind":"result","request_id":"d-01","result":{...DecisionResult...}}
  {"kind":"error","request_id":"d-01","error":"..."}
  {"kind":"summary","requests":10,"errors":0,"wall_ms":4100,
   "peak_rss_bytes":3870000000,"input_tokens":512}

`peak_rss_bytes` is this process's own high-water RSS, reported by the process
that owns the memory rather than inferred by the parent.

Nothing here talks to the network. Weights are loaded from `--model`, which may be
a local directory or a cached Hub id; under `--offline` the loader is told not to
reach out, so a missing cache fails closed instead of downloading.
"""

from __future__ import annotations

import argparse
import json
import os
import resource
import sys
import time
from typing import Any


def emit(obj: dict) -> None:
    sys.stdout.write(json.dumps(obj, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def peak_rss_bytes() -> int:
    """High-water RSS in bytes. macOS reports bytes, Linux reports KiB."""
    raw = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
    return raw if sys.platform == "darwin" else raw * 1024


def clamp_probability(value: Any) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    if number != number or number in (float("inf"), float("-inf")):
        return None
    return min(1.0, max(0.0, number))


def option_text(request: dict) -> list[str]:
    """Option descriptions in order, falling back to the id when blank.

    The contract requires a non-blank description, so the fallback is belt and
    braces rather than a normal path.
    """
    out = []
    for option in request.get("options", []):
        text = (option.get("description") or "").strip()
        out.append(text if text else str(option.get("id", "")))
    return out


def question_from_request(request: dict) -> tuple[str, dict[str, Any]]:
    """Map one contract request onto laya's question form.

    laya's vocabulary is the contract's own: `choice` takes criteria as an
    id -> description mapping, `score` takes a level list, and `noul` is a
    true/false statement whose markers are always [false, true].
    """
    kind = request["kind"]
    criterion = request.get("criterion", "")
    options = request.get("options", [])
    texts = option_text(request)

    if kind == "choice":
        criteria = {str(option["id"]): text for option, text in zip(options, texts)}
        return kind, {"type": "choice", "instructions": criterion, "criteria": criteria}

    if kind == "score":
        return kind, {"type": "score", "instructions": criterion, "criteria": texts}

    # Noul. The contract's convention is options[0] = yes, options[1] = no, and
    # laya reports the probability of the "true" marker, so the descriptions are
    # passed through in that order rather than invented here.
    criteria: dict[str, str] = {}
    if len(texts) >= 2:
        criteria = {"true": texts[0], "false": texts[1]}
    return kind, {"type": "noul", "instructions": criterion, "criteria": criteria}


def result_from_answer(request: dict, answer: dict) -> dict:
    """Map one laya answer onto the contract's DecisionResult."""
    kind = request["kind"]
    options = request.get("options", [])
    result: dict[str, Any] = {
        "request_id": request["id"],
        "prompt_sha256": request.get("prompt_sha256", ""),
    }

    if kind == "choice":
        probabilities = answer.get("probabilities") or {}
        scores = []
        for option in options:
            probability = clamp_probability(probabilities.get(str(option["id"])))
            if probability is not None:
                scores.append({"option_id": str(option["id"]), "probability": probability})
        chosen = answer.get("choice")
        if chosen is None or not any(str(o["id"]) == str(chosen) for o in options):
            raise ValueError(f"laya named an option outside the closed set: {chosen!r}")
        result["choice"] = str(chosen)
        result["scores"] = scores
        return result

    if kind == "score":
        levels = len(options)
        raw = answer.get("score")
        if raw is None:
            raise ValueError("laya returned no score")
        # laya's score is an expected level index in [0, levels - 1]. The contract
        # (and the fixture ranges) are normalised to [0, 1], so scale by the
        # widest level gap. levels >= 2 is guaranteed by the contract's option cap.
        if levels < 2:
            raise ValueError("a score request needs at least two options")
        normalised = float(raw) / float(levels - 1)
        value = clamp_probability(normalised)
        if value is None:
            raise ValueError(f"laya returned an unusable score: {raw!r}")
        result["score"] = value
        probabilities = answer.get("probabilities") or {}
        scores = []
        for index, option in enumerate(options):
            probability = clamp_probability(probabilities.get(str(index)))
            if probability is not None:
                scores.append({"option_id": str(option["id"]), "probability": probability})
        result["scores"] = scores
        return result

    # Noul: laya's `noul` is the probability of the "true" marker, which is the
    # contract's probability of yes.
    probability = clamp_probability(answer.get("noul"))
    if probability is None:
        raise ValueError(f"laya returned no usable noul probability: {answer.get('noul')!r}")
    result["noul"] = probability
    return result


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="laya decision arm child")
    parser.add_argument(
        "--model",
        default="convaiinnovations/laya",
        help="Hub id or local directory of the checkpoint",
    )
    parser.add_argument("--subfolder", default=None)
    parser.add_argument(
        "--device",
        default=None,
        help="torch device; default lets laya choose (MPS here)",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="refuse hub access; a missing cache fails closed",
    )
    parser.add_argument(
        "--backend-label",
        default=None,
        help="backend string to stamp on results; default derives it from the load",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()

    if args.offline:
        # Both switches, because either one alone still permits a head request.
        os.environ["HF_HUB_OFFLINE"] = "1"
        os.environ["TRANSFORMERS_OFFLINE"] = "1"

    started = time.time()
    import laya  # imported late so --help does not pay the torch import

    agent = laya.load(args.model, device=args.device, subfolder=args.subfolder)
    load_ms = int((time.time() - started) * 1000)
    device = str(getattr(agent, "device", "unknown"))
    label = args.backend_label or f"laya-local({args.model},{device})"
    emit({"kind": "ready", "device": device, "model": args.model, "load_ms": load_ms})

    batch_started = time.time()
    requests = 0
    errors = 0
    input_tokens = 0

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        requests += 1
        request_id = "?"
        try:
            request = json.loads(line)
            request_id = str(request.get("id", "?"))
            kind, question = question_from_request(request)
            # One request per call: laya batches internally per call, and the
            # caller owns batching, so a bad request cannot poison its siblings.
            response = agent.system_one(request.get("state", ""), {request_id: question})
            answer = (response.get("answers") or {}).get(request_id)
            if not isinstance(answer, dict):
                raise ValueError(f"laya returned no answer for {request_id!r} (kind {kind})")
            result = result_from_answer(request, answer)
            result["backend"] = label
            usage = response.get("usage") or {}
            input_tokens += int(usage.get("input_tokens") or 0)
            emit({"kind": "result", "request_id": request_id, "result": result})
        except Exception as error:  # noqa: BLE001 - every failure is reported, never swallowed
            errors += 1
            emit(
                {
                    "kind": "error",
                    "request_id": request_id,
                    "error": f"{type(error).__name__}: {error}",
                }
            )

    emit(
        {
            "kind": "summary",
            "requests": requests,
            "errors": errors,
            "wall_ms": int((time.time() - batch_started) * 1000),
            "load_ms": load_ms,
            "input_tokens": input_tokens,
            "peak_rss_bytes": peak_rss_bytes(),
            "device": device,
            "model": args.model,
        }
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
