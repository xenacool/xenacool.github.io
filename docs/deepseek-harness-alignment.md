# Enhancing the DeepSeek Harness for pystral_gate — Alignment Proposal

_Proposed: session aligned to the `pystral_gate` workspace. Scope: inspect the
workspace (guidelines, plans, PI config) and propose how to make the
DeepSeek Harness (DSH) work better with this project, aligned to its ten._

## 0. TL;DR

`pystral_gate` was configured for a **different coding agent** — the
`@earendil-works/pi-coding-agent` "pi" deployment (a.k.a. "Little Coder"). Its
safety and shell-whitelist behavior was driven by **environment variables**
(`SAFE_PREFIXES`, `LITTLE_CODER_BASH_ALLOW`, `LITTLE_CODER_PERMISSION_MODE`).

**This** harness (DeepSeek Harness) confines and approvals **differently** — by
filesystem/process wrapping controlled via `DSH_PERMISSION_MODE`, plus a
separate "ask" approval policy. It does **not** read the pi env vars. So the
project's tenets do **not** automatically apply to DSH: they must be
**re-expressed as DSH-native mechanisms**.

Two consequences matter most:

1. DSH sessions currently **cannot** run the project's own required gates —
   `make test`, `make playwright-test`, `make run-web` — because `make`/`cargo`/
   `npx`/`git`/`cd` and shell redirections are refused. To do real verification
   work here, DSH must be **enabled** to run those build tools.
2. The project's "always go through Makefile targets" and "block destructive
   commands" intents must be re-enclosed as DSH behaviors (a mode change + a
   project guardrail), not inherited from the pi env-var config.

This document maps each tenet to a DSH mechanism and specifies one **host
patch** + one **authored agent preset** to mount, respecting the host/preset
plane rule.

---

## 1. What I found

### 1.1 The project

A **WASM tactical RPG** (`crates/`, `Cargo.toml`, wasm target
`wasm32-unknown-unknown`), a browser UI (`index.html`, `game.html`,
`editor.html`, a JS worker), **Playwright** visual tests, and a **TLA+ spec**
(`spec/`). The engineering contract lives in `.junie/guidelines.md`.

**The tenets (from `.junie/guidelines.md`):**

| Intent | How it's stated |
| --- | --- |
| Ask for help; resolve **root causes**, not local patches | "Resolve root causes in principled ways rather than applying local fixes or bypassing strict checks." |
| Long-term design is the default | "Prefer the long-term design. Treat that preference as the default resolution." |
| FAQ = design debt | One authoritative name/type; low cyclomatic complexity; one typed seam; "prefer writing the focused test over the explanatory comment." |
| TLA+ first for state-machine / protocol bugs | "Inspect the existing TLA+ specification first… extend that model… run `make tla-check`… add Rust/property/browser coverage." No parallel ad-hoc models. |
| Escalate ambiguous ownership to TLA+ immediately | Rendered-but-no-input / heartbeat-but-no-progress / multi-ACK recovery → stop and refine the model. |
| **Always use Makefile targets** | "Never invoke `npx playwright test`, raw `cargo build … wasm32`, or other raw commands directly — always through Makefile targets (`make playwright-test`, `make test-browser`, `make run-web`, `make test`). `build-wasm` is self-determining about freshness." |
| Testing with a budget | "Run tests using `make test`… target captures stdout/stderr to gitignored `.make-test.log`." |
| Verification rhythm | `make test-fast` per checkpoint (TLA checks + size checks + fixture analysis); `make test` once per batch. "Never weaken a check to make the gate green." |
| No domain fallbacks | Report an error through `ui_log`; use a trivial default (0.0 / identity). |
| Fallible calls via `ui_log` | Railway-oriented pattern; log all failures. |
| Northbound/Southbound separation | Domain logic vs rendering; renderer consumes domain data, never defines it. |
| Library management | Prefer crates.io; WASM-compatible; **no global state / `static mut` / `Mutex` / `RwLock`** — message passing (`mpsc` / `WorkerBus`). |
| **Shell whitelist (the pi deployment's)** | Allowed: `ls, grep, find, python3, node`. Refused: `cd, make, cargo, npx, git` + file redirections. Enforced via `SAFE_PREFIXES` / `LITTLE_CODER_BASH_ALLOW`. |

> ⚠️ The "Shell whitelist" row above is **pi-specific**: it was enforced by the
> pi deployment through env vars. DSH does not read those. It confines by
> mode. See §3 mismatch.

### 1.2 The PI configuration (`.pi/`)

| File | Role |
| --- | --- |
| `vlm.json` | Local vision model: `http://127.0.0.1:9005/v1`, `Qwen3.8-27B-oQ4e-mtp`. Mirrors `settings.yaml` `llm-pi-ai` (`OMLX`). |
| `extensions/pg-safety.ts` | Blocks `make deploy \| nuke-deploy`, `git push --force`, `rm -rf` (hard-blocked in non-interactive mode). |
| `extensions/pg-guidelines.ts` | Injects `.junie/guidelines.md` + plan index + visual fixtures + skills into the system prompt; `/reload` re-reads. |
| `prompts/batch.md` | "Break down the next batch from `.junie/plans/index.md`; list uncertainties; **don't implement until confirmed**." |
| `skills/plan-index/SKILL.md` | Maintain `index.md` — single source of truth; ordering rules; archive, never delete. |
| `skills/visual-verify/SKILL.md` | 3-layer protocol: semantic state first (`__pystralWorkerStatus`, debug traces, settled boundary), deterministic capture (pinned seed), fixture diff ≤2%. |

### 1.3 The plan (`.junie/plans/`)

`index.md` is the single source of truth. Batches carry
**Order / Batch / Depends on / Exit**; hard rule: **do not start a later batch
while an earlier one has an unresolved invariant.** Ordering: MVP → feasibility
spike → complexity first → collate by layer (mechanics → dynamics → aesthetics).

### 1.4 The DeepSeek Harness (this one)

- Host composition = `@deepseek-ai/dsh-base` + `@deepseek-ai/dsh-web-app`.
  Currently `profiles/web/cordis.patch.yml` is **empty (`[]`)** — no project rows.
- Confinement: `dsh-sandbox-local` (Landlock/ACL) + `dsh-bash-sandbox` /
  `dsh-pwsh-sandbox`, driven by `DSH_PERMISSION_MODE`. This session: **`workspace-write`**.
- Approval: `dsh-user-approval` (this session: **`ask`**).
- Mountable capability rows include: `system-prompt`, `persona`, `skill` /
  `tool-skill`, `plan-mode`, `goal` / `goal-round-driver` / `command-goal`,
  `tool-bash` / `tool-pwsh`, `tool-subagent*`, `tool-skill`, plan/skill UI.

---

## 2. The core mismatch (why nothing "just works")

| pi deployment ("Little Coder") | DeepSeek Harness |
| --- | --- |
| Whitelist via `SAFE_PREFIXES` / `LITTLE_CODER_BASH_ALLOW` env vars | Confinement via `DSH_PERMISSION_MODE` (mode) + Landlock/ACL + "ask" approval |
| Refuses `make`/`cargo`/`npx`/`git`/`cd` statically | Refuses them under `workspace-write` too, but there's no static prefix env to edit |
| Safety extension hooks the pi `tool_call` event | DSH has no equivalent event hook in the base — must be a guardrail row (prompt/tool) or the approval policy |
| `.junie/guidelines.md` injected by `pg-guidelines.ts` (pi extension) | Must be re-enclosed as a **system-prompt section** in an authored preset |

**Bottom line:** the pi config is a *specification of intent*. To run on DSH, it
must be **compiled into DSH-native rows**: a system-prompt injection, skill
entries, a build-tool allowance, and a destructive-command guardrail. That is
exactly what this proposal specifies.

---

## 3. Mapping tenets → DSH mechanisms

| Project tenet | DSH mechanism | Plane / location | Effort |
| --- | --- | --- | --- |
| Inject `.junie/guidelines.md` + plan resources into the prompt (was `pg-guidelines.ts`) | Authored preset's **system-prompt** section, read at session start | agent preset (`pystral-gate`) | small |
| Block `make deploy/nuke-deploy`, `git push -f`, `rm -rf` (was `pg-safety.ts`) | Project **guardrail** row → route to the existing **"ask" approval** with an explicit confirmation prompt | host patch (all sessions) **or** preset (project only) | small |
| "Always use Makefile targets; never raw cargo/npx" | Enable the build-tool **allowance** in the sandbox so `make test`/`make playwright-test`/`make run-web` actually run; encode "via `make`" as a guardrail prompt | host patch + preset prompt | medium |
| Plan-index: break down batches, list uncertainties, don't implement until confirmed | **plan-mode** section + **`create_goal`** discipline (require a breakdown + uncertainty list before any edit) | preset prompt + goal tool | small |
| visual-verify: semantic-state first, deterministic capture, fixture diff ≤2% | Register the two project **skills** (`plan-index`, `visual-verify`) via `tool-skill` | preset | small |
| Verification rhythm (`test-fast` per checkpoint, `test` per batch), never weaken a check | Guardrail prompt + plan-mode checkpoints | preset prompt | small |
| TLA+ first / escalate ambiguous ownership | Guardrail prompt (mirrors the tenet) | preset prompt | small |
| Northbound/southbound, no global state, railway `ui_log` | Guardrail prompt | preset prompt | small |

---

## 4. Proposed enhancements

### 4.1 Host patch — keep the destructive guard (build tools already run)

I empirically tested this harness's sandbox: under `DSH_PERMISSION_MODE=
workspace-write` it **already allows `make`, `cargo`, `npx`, `git`, and file
redirection** (confined to the workspace filesystem, but no command-prefix
refusal). So the project's "shell whitelist" (refuse `make`/`cargo`/`npx`/`git`/
`cd`) was **entirely a pi-deployment artifact** and does **not** apply here.

Consequence: the host patch's *only* host-level job is the **destructive-command
guardrail** — because DSH, unlike the pi `pg-safety.ts`, does **not** by default
block `make deploy|nuke-deploy`, `git push --force`, or `rm -rf`. Edit
`profiles/web/cordis.patch.yml` (currently `[]`) to add, at host plane (process-global,
so every session benefits, mount-verified):

1. **Destructive-command guardrail.** Mirror `pg-safety.ts`: a system-prompt
   guardrail section that, before any bash invocation matching `make
   deploy|nuke-deploy`, `git push --force/---f`, or `rm -rf (-r/--recursive)`,
   stops and surfaces an explicit user confirmation — reusing the existing
   **`"ask"`** approval. No confinement needs loosening.

2. (Optional) reinforce the project's **"always via Makefile targets"** intent as
   a guardrail-prompt (since raw `cargo`/`npx` now *run*, remind the agent to go
   through `make <target>` and trust `build-wasm` freshness).

Because sandbox/permission rows are a **deliberate boundary**, a patch cannot
privilege *itself* beyond the deployment — but a **host** patch is the correct
place to configure a guardrail for *all* sessions, so this belongs in the host
composition, not a preset.

### 4.2 Authored agent preset — `pystral-gate`

Author a per-session preset that re-encloses the project's behavior (the DSH
analog of `pg-guidelines.ts` + `pg-safety.ts` + the two skills):

1. **Copy `standard`** (`copy(from: "standard", id: "pystral-gate")`) — it
   starts loadable and carries the full coding-agent tool set.
2. **Inject the contract** at session start: read `.junie/guidelines.md`,
   `.junie/plans/index.md`, and point at the plan resources / skills, appending
   them to the system prompt (this is `pg-guidelines.ts` re-enclosed).
3. **Register the two project skills** (`plan-index`, `visual-verify`) and the
   `batch` prompt behavior (analog of `.pi/prompts/batch.md`) via `tool-skill`.
4. **Project safety guardrail:** route `make deploy|nuke-deploy`, `git push -f`,
   `rm -rf` through an explicit confirmation.
5. **Lean into plan-mode + goals:** require a concrete, ordered task breakdown
   with file-level scope, tests, dependencies, and an explicit **uncertainty
   list** before any edit (the plan-index hard rule); use `create_goal` to carry
   multi-round batch work.
6. **Mount-verify** with `standingKeyFor("pystral-gate")`; then hand off for a
   real session.

---

## 5. Recommended sequence

1. **Decide §6** (allow build tools? destructive handled by approval only, or also a guardrail?).
2. Author the `pystral-gate` preset (`copy` from `standard`) and **mount-verify** it.
3. Apply the host patch (build-tool allowance + guardrail) — host plane, all sessions.
4. Start a session on `pystral-gate`; confirm the tool list and the injected contract.

Each step is mount-verified (`standingKeyFor`) or requires a one-time user
approval, so nothing ships unvalidated.

## 6. Decisions needed from you

The proposal has a few real forks. Your choices determine 4.1/4.2.

1. **Build tools:** already run under `workspace-write` (verified —
   `make`, `cargo`, `npx`, `git`, and redirections all execute, confined to the
   workspace). No host change needed here. (The project's shell whitelist was a
   pi-deployment artifact.)

2. **Where should the destructive-command guard live?**
   - (Recommended) **Both**: the host patch routes it through the existing
     `"ask"` approval, and the preset prompt reminds.
   - Or: **Approval policy only** (minimal rows).
   - Or: **Preset only** (project sessions get the guard; other sessions don't).

3. **Scope of the patch:** project-only (one preset) vs. all sessions (host
   patch)? The safety intent arguably applies to the whole project, favoring a
   host patch; but a host patch affects every session.

Once you answer, I'll author the `pystral-gate` preset and apply the host
patch, mount-verifying each before handing off.
