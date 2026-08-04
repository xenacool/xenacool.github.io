#!/usr/bin/env python3
"""Compaction x thinking factorial experiment driver.

Protocol: DESIGN.md. Stdlib only.

  pilot   measure base context + per-turn growth per thinking level, write calibration.json
  run     execute the (resumable) run matrix: 3 compaction x 3 thinking x R reps + sanity
  eval    recompute run.json from session JSONL + probe.txt
  report  write results.tsv + report.md with pre-registered verdicts
"""

import argparse
import json
import math
import pathlib
import random
import re
import shutil
import subprocess
import sys
import time
import urllib.request
import uuid

HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
PI_AGENT_DIR = pathlib.Path.home() / ".pi" / "agent"
VLM_JSON = REPO_ROOT / ".pi" / "vlm.json"
PI_TIMEOUT_S = 1800

SYSTEM_PROMPT = (
    "Answer directly and briefly. When asked to run a command, run it and "
    "reply with the requested word only. Do not add commentary."
)

COMPACTION = {
    "A0": {"enabled": False},
    "A1": {"enabled": True, "reserveTokens": 10000, "keepRecentTokens": 15000},
    "A2": {"enabled": True, "reserveTokens": 15000, "keepRecentTokens": 7000},
}
THINKING = ["off", "low", "medium"]
REPS = [1, 2, 3]

# 6 idiosyncratic values per replicate (rotated), unguessable by design.
FACT_SETS = [
    {
        "F1": ("NEXUS_PORT", "48231"),
        "F2": ("VAULT_CREDENTIAL", "blorx-9942"),
        "F3": ("ROTATE_ANGLE", "27.7"),
        "F4": ("LEGACY_FLAG", "ENABLE_FARCAST"),
        "F5": ("GRID_ORIGIN", "3,-5"),
        "F6": ("HANDSHAKE_TIMEOUT", "1.9"),
    },
    {
        "F1": ("NEXUS_PORT", "71052"),
        "F2": ("VAULT_CREDENTIAL", "qtitu-3180"),
        "F3": ("ROTATE_ANGLE", "63.2"),
        "F4": ("LEGACY_FLAG", "KEEP_DRAUGHT"),
        "F5": ("GRID_ORIGIN", "-6,2"),
        "F6": ("HANDSHAKE_TIMEOUT", "4.4"),
    },
    {
        "F1": ("NEXUS_PORT", "33518"),
        "F2": ("VAULT_CREDENTIAL", "vrefu-8726"),
        "F3": ("ROTATE_ANGLE", "11.9"),
        "F4": ("LEGACY_FLAG", "DROP_BEARER"),
        "F5": ("GRID_ORIGIN", "0,7"),
        "F6": ("HANDSHAKE_TIMEOUT", "0.6"),
    },
]

UNKNOWN_MARKERS = ("unknown", "unassigned", "no value", "none", "not set", "no value assigned", "did not provide")

WORD_LIST = (
    "amber braid cobalt drone ember fable gusty ivory knotty ledge mirth "
    "nodal opal quiver ridge sludge tapestry ulster vane weave yonder zephyr "
    "ashen bolt crest dune echo fen graze hush isle jolt keel luminal moss "
    "nurture ochre plume quill rafter snare tether umbrage veil whorl annex "
    "boiler corbel dimple expanse filament grating horizon ingress junction "
    "keystone lattice meridian nozzle ossify precipice quorum rioter saunter "
    "tessellate undulate vortex wainscot auburn baleful canopy digitize "
    "exhume forage galvanize holler incense jostle kindle loom molt nuzzle "
    "oracle pierce quest relay scaffold thaw unpack vibrate waltz abacus "
    "baffle cinder drumming eddy folio granary halcyon ironwork jubilee "
    "kiln leaflet mortar nettle osmotic pyramid quartz rhodochrosite silt "
    "turbine udder vellum warpful ascot bigwig clatter dozing enigma "
    "fathomed gossamer haberdashery ingot jigjig kelpie lozenge marmalade "
    "noisome opulent prowler rostrum sorrel tallow uncanny vagrant waterlily "
    "append bough candor direness enfilade fluvial gibberish hoarding "
    "insinuate jumble kowtowl limber marmoset nimble overture pincers "
    "quadrant ruminative sconce thrifty unfurl vintner wistfulness abeyance "
    "byzantine crenulate dermal equilibrium flibbertigibbet gristhouse "
    "hurtle interstice jettison kennel lullaby minuet nascent onyx pledge "
    "quicksilver riddle shadeless thrum umber vagrant waterfall"
).split()


def default_provider_model():
    cfg = json.loads((PI_AGENT_DIR / "settings.json").read_text())
    return cfg.get("defaultProvider", "omlx"), cfg.get("defaultModel")


def gen_filler(seed: int, kb: int = 32) -> str:
    rng = random.Random(seed)
    out, size = [], 0
    while size < kb * 1024:
        w = rng.choice(WORD_LIST)
        out.append(w)
        size += len(w) + 1
    return " ".join(out)


def setup_prompt(facts) -> str:
    lines = "\n".join(f"{k} = {v}" for k, (_, v) in facts.items())
    return (
        "Remember these six configuration values exactly. Later I will ask you "
        "to write each one down from memory:\n"
        f"{lines}\n"
        "Also: PHANTOM_KEY is intentionally unassigned; it has no value.\n"
        "Reply with exactly: ready"
    )


def filler_prompt(i: int) -> str:
    return f"Run exactly this bash command: cat filler/part_{i}.txt\nReply with exactly: ok"


PROBE_PROMPT = (
    "From memory, write down the six configuration values I gave you earlier, "
    "one per line, exactly in this format:\n"
    "F1: <NEXUS_PORT>\n"
    "F2: <VAULT_CREDENTIAL>\n"
    "F3: <ROTATE_ANGLE>\n"
    "F4: <LEGACY_FLAG>\n"
    "F5: <GRID_ORIGIN>\n"
    "F6: <HANDSHAKE_TIMEOUT>\n"
    "F7: <PHANTOM_KEY, if it has a value>\n"
    "If a value is unknown to you, write UNKNOWN after the colon. Nothing else."
)


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", s.strip().lower().rstrip(".,;:"))


def value_match(candidate: str, truth: str) -> bool:
    c = norm(candidate)
    if c == norm(truth):
        return True
    try:
        return abs(float(c) - float(truth)) < 1e-6
    except ValueError:
        return False


def is_unknown(candidate: str) -> bool:
    c = norm(candidate)
    return c == "" or any(m in c for m in UNKNOWN_MARKERS)


# ---------------------------------------------------------------- session JSONL


def find_session(sandbox: pathlib.Path, sid: str) -> pathlib.Path | None:
    for p in sorted((sandbox / "sessions").glob(f"*_{sid}.jsonl")):
        return p
    return None


def parse_session(path: pathlib.Path) -> dict:
    usage_list, compactions, stop_reasons = [], [], []
    thinking_chars = 0
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        t = entry.get("type")
        if t == "compaction":
            compactions.append(
                {
                    "summary": entry.get("summary", ""),
                    "tokensBefore": entry.get("tokensBefore"),
                    "firstKeptEntryId": entry.get("firstKeptEntryId"),
                }
            )
        elif t == "message":
            msg = entry.get("message", entry)
            usage = msg.get("usage") or entry.get("usage")
            if msg.get("role") == "assistant":
                if usage:
                    usage_list.append(usage)
                stop_reasons.append(msg.get("stopReason"))
                for block in msg.get("content", []):
                    if isinstance(block, dict) and block.get("type") == "thinking":
                        thinking_chars += len(block.get("thinking", ""))
    return {
        "usage": usage_list,
        "compactions": compactions,
        "stop_reasons": stop_reasons,
        "thinking_chars": thinking_chars,
    }


# -------------------------------------------------------------------- probing


def llm_judge(truth: str, candidate: str) -> bool | None:
    """Tier-2 judge on the local omlx endpoint (vision-capable, used text-only)."""
    if not VLM_JSON.exists():
        return None
    vlm = json.loads(VLM_JSON.read_text())
    body = {
        "model": vlm["model"],
        "temperature": 0,
        "max_tokens": 16,
        "messages": [
            {
                "role": "user",
                "content": (
                    f"Correct value: {truth}\n"
                    f"Candidate answer: {candidate}\n"
                    "Is the candidate conveying the correct value (format differences are OK)? "
                    "Reply with only the word yes or no."
                ),
            }
        ],
    }
    req = urllib.request.Request(
        vlm["endpoint"].rstrip("/") + "/chat/completions",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {vlm.get('api_key', '')}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            out = json.loads(r.read())
        text = norm(out["choices"][0]["message"]["content"])
        if text.startswith("no"):
            return False
        if text.startswith("yes"):
            return True
        return None
    except Exception:
        return None


def score_probe(probe_text: str, facts: dict) -> dict:
    answers = {}
    for m in re.finditer(r"^F(\d):\s*(.*)$", probe_text, re.MULTILINE):
        answers[f"F{m.group(1)}"] = m.group(2).strip()
    results, correct, judge_used = {}, 0, 0
    for fid, (key, truth) in facts.items():
        raw = answers.get(fid, "")
        judged = None
        if value_match(raw, truth):
            ok = True
        elif is_unknown(raw):
            ok = False
        else:
            judge_used += 1
            judged = llm_judge(truth, raw)
            ok = bool(judged)
        correct += 1 if ok else 0
        results[fid] = {"key": key, "raw": raw, "correct": ok, "judged": judged}
    phantom_raw = answers.get("F7", "")
    phantom_hallucination = bool(phantom_raw) and not is_unknown(phantom_raw)
    return {
        "probe_correct": correct,
        "results": results,
        "phantom": {"raw": phantom_raw, "hallucinated": phantom_hallucination},
        "judge_used": judge_used,
    }


def summary_fact_hits(summary: str, facts: dict) -> int:
    s = norm(summary)
    return sum(1 for _, (_, v) in facts.items() if v.lower() in s)


# ----------------------------------------------------------------------- pi


def pi_base_args(pi_bin, provider, model, thinking, sid, tools):
    args = [
        pi_bin, "-p", "--offline", "-ne", "-ns", "-np", "-nc",
        "--no-themes", "--no-approve",
        "--system-prompt", SYSTEM_PROMPT,
        "--provider", provider, "--model", model,
        "--thinking", thinking,
        "--session-dir", "sessions", "--session-id", sid,
    ]
    if tools:
        args += ["--tools", "bash"]
    else:
        args += ["--no-tools"]
    return args


def run_pi(pi_bin, provider, model, thinking, sid, sandbox, prompts, out_name, timeout=PI_TIMEOUT_S):
    args = pi_base_args(pi_bin, provider, model, thinking, sid, tools=bool(prompts))
    args += prompts
    t0 = time.monotonic()
    try:
        proc = subprocess.run(
            args, cwd=sandbox, capture_output=True, text=True, timeout=timeout
        )
        status, stderr = ("ok" if proc.returncode == 0 else f"pi_exit_{proc.returncode}"), proc.stderr
    except subprocess.TimeoutExpired as e:
        status = "timeout"
        stderr = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
    (sandbox / out_name).write_text(stderr or "")
    return status, stderr or "", time.monotonic() - t0


def stuck_marker(stderr: str) -> bool:
    return any(m in stderr for m in (
        "compaction could not proceed", "Already compacted", "paused to avoid a loop"
    ))


# ------------------------------------------------------------------ run cells


def make_sandbox(cell_dir: pathlib.Path, cond: str, n: int, seed: int) -> pathlib.Path:
    sandbox = cell_dir / "sandbox"
    (sandbox / ".pi").mkdir(parents=True, exist_ok=True)
    (sandbox / ".pi" / "settings.json").write_text(json.dumps({"compaction": COMPACTION[cond]}))
    filler = sandbox / "filler"
    filler.mkdir(exist_ok=True)
    for i in range(1, n + 1):
        (filler / f"part_{i}.txt").write_text(gen_filler(seed * 100 + i))
    (sandbox / "sessions").mkdir(exist_ok=True)
    return sandbox


def run_cell(args, resdir, cond, thinking, rep, n_override=None, sanity=False):
    label = f"{cond}/{thinking}/rep-{rep}" + ("-sanity" if sanity else "")
    cell_dir = resdir / ("sanity" if sanity else "runs") / cond / thinking / f"rep-{rep}"
    if (cell_dir / "run.json").exists() and not args.rerun:
        print(f"[skip] {label}")
        return json.loads((cell_dir / "run.json").read_text())
    cell_dir.mkdir(parents=True, exist_ok=True)

    provider, model = default_provider_model()
    if args.provider:
        provider, model = args.provider, args.model or model
    cal = json.loads((resdir / "calibration.json").read_text()) if (resdir / "calibration.json").exists() else {}
    n = n_override if n_override is not None else (cal.get(thinking, {}).get(f"n_{cond}") or 6)
    if sanity:
        n = 0
    facts = FACT_SETS[(rep - 1) % len(FACT_SETS)]
    sid = str(uuid.uuid4())
    seed = (rep - 1) * 1000 + 7
    sandbox = make_sandbox(cell_dir, cond, max(n, 1), seed)

    wall, status, stderr_all = 0.0, "ok", ""
    prompts = [setup_prompt(facts)] + [filler_prompt(i) for i in range(1, n + 1)]
    s1, err1, w1 = run_pi(args.pi, provider, model, thinking, sid, sandbox, prompts, "filler_stderr.txt")
    wall += w1
    stderr_all += err1
    if s1 != "ok":
        status = s1
    s2, err2, w2 = run_pi(args.pi, provider, model, thinking, sid, sandbox, [PROBE_PROMPT], "probe_stdout.txt")
    wall += w2
    stderr_all += err2
    probe_text = ""
    p = sandbox / "probe_stdout.txt"
    # probe run wrote stderr to probe_stdout.txt; stdout is discarded by run_pi.
    # Capture probe text from the session instead if so (see eval). Fallback: none.
    if status == "ok":
        status = s2 if s2 != "ok" else "ok"

    session = find_session(sandbox, sid)
    run = {
        "cell": {"compaction": cond, "thinking": thinking, "sanity": sanity},
        "rep": rep,
        "fact_set": (rep - 1) % len(FACT_SETS),
        "n_filler": n,
        "status": status,
        "wall_s": round(wall, 1),
        "compaction_stuck": stuck_marker(stderr_all),
        "provider": provider,
        "model": model,
        "session_file": str(session.relative_to(resdir)) if (session and resdir in session.parents) else None,
    }
    if session:
        cell_dir.joinpath("session.jsonl").write_bytes(session.read_bytes())
    if status == "ok":
        run.update(eval_run(cell_dir, resdir, facts, probe_text or None))
    (cell_dir / "run.json").write_text(json.dumps(run, indent=2))
    print(f"[done] {label}: status={run.get('status')} probe={run.get('probe_correct')}/6 stuck={run.get('compaction_stuck')}")
    return run


def eval_run(cell_dir, resdir, facts, probe_text=None):
    """Recompute measurements from session.jsonl; optional probe text override."""
    sess = cell_dir / "session.jsonl"
    out = {}
    if sess.exists():
        parsed = parse_session(sess)
        out["peak_context"] = max(
            (u.get("totalTokens") or 0 for u in parsed["usage"]), default=0
        )
        out["thinking_tokens"] = sum(u.get("reasoning", 0) for u in parsed["usage"])
        out["api_errors"] = sum(1 for r in parsed["stop_reasons"] if r == "error")
        out["n_compactions"] = len(parsed["compactions"])
        if parsed["compactions"]:
            last = parsed["compactions"][-1]
            out["compaction_tokens_before"] = last["tokensBefore"]
            out["summary_fact_hits"] = summary_fact_hits(last["summary"], facts)
        else:
            out["compaction_tokens_before"] = None
            out["summary_fact_hits"] = None
    # probe text: prefer provided, else re-extract last assistant text from session
    if probe_text is None:
        probe_text = extract_last_text(sess) if sess.exists() else ""
    scored = score_probe(probe_text or "", facts)
    out.update(scored)
    return out


def extract_last_text(sess: pathlib.Path) -> str:
    last = ""
    for line in sess.read_text().splitlines():
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if entry.get("type") == "message":
            msg = entry.get("message", entry)
            if msg.get("role") == "assistant":
                parts = [b.get("text", "") for b in msg.get("content", [])
                         if isinstance(b, dict) and b.get("type") == "text"]
                if parts:
                    last = "\n".join(parts)
    return last


# --------------------------------------------------------------------- pilot


def run_pilot(args, resdir):
    """A1 config, one run per thinking level with 3 filler turns; derive base+growth."""
    provider, model = (args.provider, args.model) if args.provider else default_provider_model()
    cal = {}
    for thinking in THINKING:
        cell_dir = resdir / "pilot" / thinking
        cell_dir.mkdir(parents=True, exist_ok=True)
        if (cell_dir / "pilot.json").exists() and not args.rerun:
            cal[thinking] = json.loads((cell_dir / "pilot.json").read_text())
            continue
        sandbox = make_sandbox(cell_dir, "A1", 3, seed=99)
        sid = str(uuid.uuid4())
        facts = FACT_SETS[0]
        prompts = [setup_prompt(facts)] + [filler_prompt(i) for i in range(1, 4)]
        status, err, wall = run_pi(args.pi, provider, model, thinking, sid, sandbox, prompts, "stderr.txt")
        session = find_session(sandbox, sid)
        entry = {"status": status, "wall_s": round(wall, 1)}
        if session:
            cell_dir.joinpath("session.jsonl").write_bytes(session.read_bytes())
            parsed = parse_session(session)
            totals = [u.get("totalTokens") for u in parsed["usage"] if u.get("totalTokens")]
            if len(totals) >= 3:
                # first total = setup turn; next 3 = filler turns
                base = totals[0]
                growths = [totals[i + 1] - totals[i] for i in range(len(totals) - 1)]
                entry.update({
                    "base_tokens": base,
                    "growth_per_turn": int(sum(growths) / len(growths)),
                    "totals": totals,
                })
                cal[thinking] = entry
        (cell_dir / "pilot.json").write_text(json.dumps(entry, indent=2))
        print(f"[pilot] {thinking}: {entry}")

    # derive N per cell
    plan = {}
    for thinking, e in cal.items():
        if "base_tokens" not in e:
            continue
        base, growth = e["base_tokens"], max(e["growth_per_turn"], 1)
        n_a1 = max(1, min(12, math.ceil((40000 - base) / growth)))
        n_a2 = max(1, min(12, math.ceil((35000 - base) / growth)))
        plan[thinking] = {
            **e,
            "n_A0": n_a1, "n_A1": n_a1, "n_A2": n_a2,
            "projected_A0": base + n_a1 * growth,
            "expect_overflow_A0": (base + n_a1 * growth) >= 48000,
        }
    (resdir / "calibration.json").write_text(json.dumps(plan, indent=2))
    print(f"[pilot] calibration written: {json.dumps(plan, indent=2)}")
    return plan


def default_n(cal: dict, thinking: str, cond: str) -> int:
    e = cal.get(thinking, {})
    return e.get(f"n_{cond}") or 6


# -------------------------------------------------------------------- report


def load_runs(resdir: pathlib.Path):
    runs = []
    for p in sorted((resdir / "runs").glob("*/*/*/run.json")):
        r = json.loads(p.read_text())
        r["_path"] = str(p.relative_to(resdir))
        runs.append(r)
    return runs


def cell_stats(runs, cond, thinking):
    sel = [r for r in runs if r["cell"]["compaction"] == cond and r["cell"]["thinking"] == thinking
           and r.get("status") == "ok"]
    if not sel:
        return None
    return {
        "n": len(sel),
        "probe": sum(r["probe_correct"] for r in sel) / len(sel),
        "stuck": sum(1 for r in sel if r.get("compaction_stuck")),
        "errors": sum(1 for r in sel if r.get("api_errors", 0) > 0),
        "compactions": sum(r.get("n_compactions", 0) for r in sel),
        "summary_hits": sum(r.get("summary_fact_hits") or 0 for r in sel) / max(1, sum(1 for r in sel if r.get("summary_fact_hits") is not None)),
        "thinking_tokens": sum(r.get("thinking_tokens", 0) for r in sel) / len(sel),
        "wall": sum(r.get("wall_s", 0) for r in sel) / len(sel),
    }


def write_report(args, resdir):
    runs = load_runs(resdir)
    rows = [r for r in runs]
    tsv = resdir / "results.tsv"
    cols = ["compaction", "thinking", "rep", "status", "n_filler", "probe_correct",
            "summary_fact_hits", "n_compactions", "compaction_tokens_before", "peak_context",
            "thinking_tokens", "api_errors", "compaction_stuck", "phantom_hallucinated",
            "judge_used", "wall_s"]
    with tsv.open("w") as f:
        f.write("\t".join(cols) + "\n")
        for r in rows:
            vals = [
                r["cell"]["compaction"], r["cell"]["thinking"], r["rep"], r.get("status"),
                r.get("n_filler"), r.get("probe_correct"), r.get("summary_fact_hits"),
                r.get("n_compactions"), r.get("compaction_tokens_before"), r.get("peak_context"),
                r.get("thinking_tokens"), r.get("api_errors"), r.get("compaction_stuck"),
                r["phantom"]["hallucinated"] if "phantom" in r else None,
                r.get("judge_used"), r.get("wall_s"),
            ]
            f.write("\t".join("" if v is None else str(v) for v in vals) + "\n")

    lines = ["# Compaction x thinking — results", "", f"runs: {len(rows)}", ""]
    header = ["cell", "runs", "probe/6", "sumHits/6", "compact", "stuck", "errs", "thinkTok", "wall_s"]
    lines.append("| " + " | ".join(header) + " |")
    lines.append("|" + "---|" * len(header))
    for cond in COMPACTION:
        for thinking in THINKING:
            s = cell_stats(runs, cond, thinking)
            if s:
                lines.append(
                    f"| {cond}/{thinking} | {s['n']} | {s['probe']:.2f} | {s['summary_hits']:.2f} "
                    f"| {s['compactions']} | {s['stuck']} | {s['errors']} | {s['thinking_tokens']:.0f} | {s['wall']:.0f} |"
                )
            else:
                lines.append(f"| {cond}/{thinking} | 0 | - | - | - | - | - | - | - |")

    def pairs(cond_a, cond_b, thinking):
        a = {r["rep"]: r.get("probe_correct") for r in runs
             if r["cell"]["compaction"] == cond_a and r["cell"]["thinking"] == thinking and r.get("status") == "ok"}
        b = {r["rep"]: r.get("probe_correct") for r in runs
             if r["cell"]["compaction"] == cond_b and r["cell"]["thinking"] == thinking and r.get("status") == "ok"}
        common = set(a) & set(b)
        return [(a[x], b[x]) for x in sorted(common)]

    lines += ["", "## Verdicts (pre-registered rules, see DESIGN.md)", ""]
    for thinking in THINKING:
        ps = pairs("A1", "A0", thinking)
        if len(ps) >= 2:
            worse = sum(1 for a, b in ps if a < b)
            mean_diff = sum(a - b for a, b in ps) / len(ps)
            supp = worse >= 2 and mean_diff <= -1.0
            lines.append(
                f"- H1 ({thinking}): A1 worse in {worse}/{len(ps)} pairs, mean diff {mean_diff:+.2f} → "
                f"{'SUPPORTED (descriptive, n=' + str(len(ps)) + ')' if supp else 'not supported'}"
            )
    a1 = [r for r in runs if r["cell"]["compaction"] == "A1" and r.get("status") == "ok"]
    a2 = [r for r in runs if r["cell"]["compaction"] == "A2" and r.get("status") == "ok"]
    if a1 and a2:
        m1, m2 = (sum(r["probe_correct"] for r in a) / len(a) for a in (a1, a2))
        s1 = sum(1 for r in a1 if r.get("compaction_stuck"))
        s2 = sum(1 for r in a2 if r.get("compaction_stuck"))
        supp = m2 >= m1 and s2 <= s1
        lines.append(f"- H2: A2 probe {m2:.2f} vs A1 {m1:.2f}, stuck {s2} vs {s1} → {'SUPPORTED' if supp else 'not supported'}")
    for thinking in THINKING:
        sel = [r for r in a1 if r["cell"]["thinking"] == thinking]
        if len(sel) >= 2:
            lines.append(
                f"- H3 ({thinking}): A1 compactions/run {sum(r.get('n_compactions', 0) for r in sel) / len(sel):.2f}, "
                f"thinking_tokens {sum(r.get('thinking_tokens', 0) for r in sel) / len(sel):.0f}"
            )
    sanity = [json.loads(p.read_text()) for p in sorted((resdir / "sanity").glob("*/*/run.json"))] if (resdir / "sanity").exists() else []
    if sanity:
        ok = all(r.get("probe_correct", 0) >= 6 for r in sanity if r.get("status") == "ok")
        lines.append(f"- Sanity gate: {'PASS' if ok else 'FAIL — comparisons are relative only'}")
    (resdir / "report.md").write_text("\n".join(lines) + "\n")
    print(f"[report] {resdir / 'report.md'}")


# --------------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["pilot", "run", "eval", "report"])
    ap.add_argument("--results-dir", type=pathlib.Path, default=None,
                    help="reuse an existing results dir (else a new timestamped one)")
    ap.add_argument("--pi", default=None, help="path to pi binary (default: which pi)")
    ap.add_argument("--provider", default=None)
    ap.add_argument("--model", default=None)
    ap.add_argument("--quick", action="store_true", help="2x2, 1 rep, filler capped at 5")
    ap.add_argument("--rerun", action="store_true")
    args = ap.parse_args()

    args.pi = args.pi or shutil.which("pi") or "pi"
    if args.results_dir:
        resdir = args.results_dir
    else:
        stamp = time.strftime("%Y%m%d-%H%M%S")
        resdir = HERE / "results" / stamp
    resdir.mkdir(parents=True, exist_ok=True)

    if args.quick:
        global THINKING, REPS, COMPACTION
        THINKING = ["off", "low"]
        REPS = [1]
        COMPACTION = {k: v for k, v in COMPACTION.items() if k in ("A0", "A1")}

    if args.cmd == "pilot":
        run_pilot(args, resdir)
    elif args.cmd == "run":
        if args.quick:
            run_pilot(args, resdir)
        cal = json.loads((resdir / "calibration.json").read_text()) if (resdir / "calibration.json").exists() else {}
        for thinking in THINKING:
            for cond in COMPACTION:
                n = default_n(cal, thinking, cond) if not args.quick else min(5, default_n(cal, thinking, cond))
                for rep in REPS:
                    run_cell(args, resdir, cond, thinking, rep, n_override=n)
            for rep in REPS:
                run_cell(args, resdir, "A1", thinking, rep, n_override=0, sanity=True)
        write_report(args, resdir)
    elif args.cmd == "eval":
        for p in sorted((resdir / "runs").glob("*/*/*/run.json")):
            r = json.loads(p.read_text())
            facts = FACT_SETS[r["fact_set"]]
            upd = eval_run(p.parent, resdir, facts)
            r.update(upd)
            p.write_text(json.dumps(r, indent=2))
            print(f"[eval] {p}")
        write_report(args, resdir)
    elif args.cmd == "report":
        write_report(args, resdir)


if __name__ == "__main__":
    main()
