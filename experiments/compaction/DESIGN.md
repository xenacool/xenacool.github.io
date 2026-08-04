# Compaction × Thinking Factorial Experiment

Pre-registered protocol. **Decision rules below are fixed before the full matrix runs.**
Pilot runs calibrate filler volume only; they are not outcome data.

## Question

Does the local pi compaction configuration (`reserveTokens` / `keepRecentTokens` / `enabled`)
degrade verifiable outputs, and does the thinking level interact with that effect?
Motivated by issue #68 ("Already compacted — paused to avoid a loop").

## Model & harness (constant across all runs)

- Model: whatever `defaultProvider`/`defaultModel` say in `~/.pi/agent/settings.json`
  (currently omlx `qwen3.8-27b-mlx`, 50k context window, local omlx server).
- Fixed minimal `--system-prompt` (2 sentences). The little-coder persona, skills,
  extensions, context files, themes, and prompt templates are all disabled
  (`-ne -ns -np -nc --no-themes --offline`) so the 50k window is available for filler.
  Conclusions are about the *compaction mechanism*, not the full coding-agent harness.
- Filler run: `--tools bash` (model cats deterministic ~32KB filler files).
- Probe run: separate `pi` invocation resuming the same session with `--no-tools`,
  so the answer comes only from context.
- Sandbox per run: temp dir with `.pi/settings.json` containing **only** the compaction
  block under test; everything else deep-merges over the user's real global settings.
  The user's live `settings.json` is never modified.

## Factors

**Factor A — compaction config** (written into sandbox `.pi/settings.json`):
- `A0`: `{"enabled": false}` — control. Verified in source: with `enabled: false`
  neither threshold compaction nor overflow-recovery compaction fires
  (`_checkCompaction` returns false at the gate).
- `A1` = current local settings: `{"enabled": true, "reserveTokens": 10000, "keepRecentTokens": 15000}`
  → fires when context > 40k.
- `A2` = proposed tuning: `{"enabled": true, "reserveTokens": 15000, "keepRecentTokens": 7000}`
  → fires when context > 35k, keeps less.

**Factor B — thinking level** (`--thinking`): `off`, `low`, `medium`.
(`high`/`max` map to null in this model's `thinkingLevelMap`; `xhigh` too slow locally — add later if needed.)

**Cells**: 3 × 3 = 9, × 3 replicates = 27 runs + 3 sanity runs (C3, no filler) = 30 runs.

## Task

1. **Setup turn**: model memorizes 6 idiosyncratic key/value pairs (e.g. `NEXUS_PORT = 48231`)
   plus is told `PHANTOM_KEY` is intentionally unassigned. Values are unguessable and
   rotated per replicate (3 pre-generated fact sets) to prevent "memorized one value" confounds.
2. **Filler turns** (N per cell, calibrated by pilot): model runs `cat filler/part_i.txt`
   and replies "ok". Grows context ~8–14k tokens/turn.
3. **Probe turn**: model writes `F1..F7: <value>` from memory, `UNKNOWN` if not known.

### N (filler turns) per cell

Pilot measures base context and per-turn growth per thinking level, then:
- `A0`, `A1`: N = smallest N such that projected context crosses the **40k** A1 threshold.
  (A0 and A1 reach the same context size before any compaction → fair comparison.)
- `A2`: N = smallest N such that projected context crosses the **35k** A2 threshold.
- If A0 would exceed 49k at that N, the cell is marked `expect_overflow` and an overflow
  is a *recorded outcome*, not a failure.

## Measurements (all machine-verified from session JSONL + probe output)

| var | source |
|---|---|
| `n_compactions` | count of `type:"compaction"` entries (treatment receipt: A1/A2 must be ≥ 1) |
| `compaction_tokens_before` | last compaction entry's `tokensBefore` |
| `peak_context` | max `usage.totalTokens` over assistant messages |
| `summary_fact_hits` | of 6 fact values, how many appear verbatim in the last compaction summary |
| `probe_correct` | of 6 facts, how many exactly right (normalized) |
| `phantom_hallucination` | model asserted a value for PHANTOM_KEY |
| `judge_used` / `judge_correct` | tier-2 LLM judge (omlx endpoint from `.pi/vlm.json`, text-only, temp 0) invoked only when exact match fails and answer is non-UNKNOWN |
| `thinking_tokens` | Σ `usage.reasoning` over the run |
| `api_errors` | assistant messages with `stopReason:"error"` |
| `compaction_stuck` | stderr matches "compaction could not proceed" / "Already compacted" — the #68 symptom |
| `wall_s` | filler + probe wall clock |

## Hypotheses & decision rules (fixed in advance)

- **H1 (compaction is harmful)**: at each thinking level, mean `probe_correct(A1) <
  probe_correct(A0)` by ≥ 1 fact, with `summary_fact_hits(A1) < 6` in a majority of runs.
  Tested per thinking level with a sign test over the 3 replicate pairs (exact p reported).
  **Falsified** if A1 ≥ A0 − 0 on all thinking levels.
- **H2 (tuning mitigates)**: mean `probe_correct(A2) ≥ mean probe_correct(A1)` AND
  `compaction_stuck(A2) ≤ stuck(A1)` AND `api_errors(A2) ≤ api_errors(A1)`.
- **H3 (thinking × compaction interaction)**: within A1, `n_compactions` and
  `compaction_stuck` increase monotonically with thinking level; `thinking_tokens`
  reported as the mediator.
- **Sanity gate (C3)**: if any no-filler sanity run scores < 6/6, memorization is
  unreliable and all comparisons are reported as relative only.
- **Falsification of the design**: if in *any* A1 cell `n_compactions == 0` (compaction
  never fired — treatment receipt failure), the run is marked `invalid`, not scored.
  If all 9 A1 cells fail receipt, the experiment is void.
- Small-n note: n=3 per cell, so decisions use exact sign-test p-values and raw counts,
  not parametric tests. Report everything; interpret with that caveat.

## Run budget

~12–16 turns/run; local 27B ≈ 15–45 s/turn → ~5–10 min/run → full matrix ≈ 2.5–5 h.
Smoke test: `--quick` (A×{off,low}, 1 replicate) ≈ 45 min.

## Files

- `run_experiment.py` — driver. Subcommands: `pilot` (calibrate), `run` (matrix,
  resumable), `eval` (recompute run.json from session files), `report` (results.tsv +
  report.md with verdicts).
- `results/<timestamp>/` — sandboxes, session JSONL copies, probe outputs, run.json,
  results.tsv, report.md. (gitignored)
