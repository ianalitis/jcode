#!/usr/bin/env python3
"""A stdlib-only stand-in for the laya child, used to test the process boundary.

It speaks the same JSON-lines protocol as `laya_arm_child.py` and answers
deterministically without torch, so the transport, the timeout, the environment
scrubbing and every fail-closed path can be tested without a model. It is
deliberately able to misbehave: see `--behaviour`.

It also prints its own environment variable names to stderr as
`ENV_NAMES=...`, which is how the tests assert that no credential reaches the
child.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time


def emit(obj: dict) -> None:
    sys.stdout.write(json.dumps(obj, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def answer_for(request: dict) -> dict:
    kind = request["kind"]
    options = request.get("options", [])
    if kind == "choice":
        probabilities = {}
        for index, option in enumerate(options):
            # A stand-in distribution: the first option wins, the rest split the
            # remainder. Never a remark about the state, so it is stable.
            probabilities[str(option["id"])] = 0.9 if index == 0 else 0.1 / max(
                1, len(options) - 1
            )
        return {
            "type": "choice",
            "choice": str(options[0]["id"]),
            "probabilities": probabilities,
            "confidence": 0.9,
            "action": {"act_probability": 0.0},
        }
    if kind == "score":
        levels = len(options)
        return {
            "type": "score",
            "score": (levels - 1) / 2.0,
            "probabilities": {str(i): 1.0 / levels for i in range(levels)},
            "confidence": 0.5,
            "action": {"act_probability": 0.0},
        }
    return {
        "type": "noul",
        "noul": 0.9,
        "confidence": 0.9,
        "action": {"act_probability": 0.0},
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="stub laya arm child")
    parser.add_argument("--model", default="stub")
    parser.add_argument("--subfolder", default=None)
    parser.add_argument("--device", default=None)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--backend-label", default=None)
    parser.add_argument(
        "--behaviour",
        default="ok",
        choices=[
            "ok",
            "crash",
            "hang",
            "garbage",
            "unknown-option",
            "summarise-early",
            "huge-rss",
        ],
    )
    parser.add_argument("--peak-rss-bytes", type=int, default=64 * 1024 * 1024)
    args = parser.parse_args()

    # The parent is expected to have scrubbed this; the stub reports what it saw.
    print("ENV_NAMES=" + ",".join(sorted(os.environ)), file=sys.stderr, flush=True)

    if args.behaviour == "crash":
        print("stub: crashing before ready", file=sys.stderr, flush=True)
        return 3

    emit(
        {
            "kind": "ready",
            "device": args.device or "stub",
            "model": args.model,
            "load_ms": 1,
            "pid": os.getpid(),
        }
    )

    if args.behaviour == "hang":
        time.sleep(3600)
        return 0

    requests = 0
    errors = 0
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        requests += 1
        if args.behaviour == "summarise-early":
            break
        if args.behaviour == "garbage":
            sys.stdout.write("this is not JSON\n")
            sys.stdout.flush()
            continue
        request = json.loads(line)
        answer = answer_for(request)
        if args.behaviour == "unknown-option":
            answer["choice"] = "not-an-option"
        result = {
            "request_id": request["id"],
            "prompt_sha256": request.get("prompt_sha256", ""),
            "backend": "stub",
        }
        if request["kind"] == "choice":
            result["choice"] = answer["choice"]
            result["scores"] = [
                {"option_id": key, "probability": value}
                for key, value in answer["probabilities"].items()
            ]
        elif request["kind"] == "score":
            result["score"] = answer["score"]
        else:
            result["noul"] = answer["noul"]
        emit({"kind": "result", "request_id": request["id"], "result": result})

    peak = args.peak_rss_bytes
    if args.behaviour == "huge-rss":
        peak = 64 * 1024 * 1024 * 1024
    emit(
        {
            "kind": "summary",
            "requests": requests,
            "errors": errors,
            "wall_ms": 1,
            "load_ms": 1,
            "input_tokens": 0,
            "peak_rss_bytes": peak,
            "device": args.device or "stub",
            "model": args.model,
            "pid": os.getpid(),
        }
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
