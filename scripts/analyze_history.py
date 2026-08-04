#!/usr/bin/env python3
"""Small, dependency-free analyzer for downloaded Pystral history diagnostics."""

import argparse
import json
from pathlib import Path


def analyze(payload):
    history = payload.get("history", payload)
    events = history.get("log", history.get("events", []))
    replay = payload.get("replay", {})
    diagnostics = payload.get("diagnostics", {})
    moves = [event["MoveSprite"] for event in events if "MoveSprite" in event]
    transitions = [
        move for move in moves if move.get("transition") is not None
    ]
    current_index = history.get("current_index")
    sequence_numbers = [
        event["SequenceNumber"] for event in events if "SequenceNumber" in event
    ]
    latest_input = diagnostics.get("workerLatestInputSeq")
    latest_output = diagnostics.get("workerLatestOutputSeq")
    input_output_gap = (
        latest_input - latest_output
        if isinstance(latest_input, int) and isinstance(latest_output, int)
        else None
    )
    return {
        "event_count": len(events),
        "move_events": len(moves),
        "transition_events": len(transitions),
        "current_index": current_index,
        "batched_event_gap": (
            current_index is not None and current_index > 1 and len(events) > 1
        ),
        "sequence_numbers": sequence_numbers,
        "last_sequence_number": sequence_numbers[-1] if sequence_numbers else None,
        "tail_event_types": [next(iter(event), "Unknown") for event in events[-12:]],
        "worker_status": diagnostics.get("workerStatus"),
        "worker_latest_input_seq": latest_input,
        "worker_latest_output_seq": latest_output,
        "input_output_gap": input_output_gap,
        "possible_control_input_flood": (
            input_output_gap is not None and input_output_gap > 100
        ),
        "replay_entrypoint": replay.get("entrypoint"),
        "replay_action_input_count": len(replay.get("actionInputs", [])),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    payload = json.loads(args.path.read_text())
    result = analyze(payload)
    print(json.dumps(result, sort_keys=True))

    expected = payload.get("expected")
    if expected:
        for key, value in expected.items():
            if result.get(key) != value:
                raise SystemExit(
                    f"{args.path}: expected {key}={value!r}, got {result.get(key)!r}"
                )


if __name__ == "__main__":
    main()
