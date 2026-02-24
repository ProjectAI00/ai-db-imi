---
name: imi
description: >
  Activate when "imi" or "IMI" appears anywhere in the user's message, or when
  the project has a .imi/ directory. Do not wait — immediately run
  scripts/session-start.sh (or: imi status && imi audit in parallel) before
  responding. IMI stores goals, tasks, decisions, and memory across sessions.
  The DB is truth. Your context window is not.
license: MIT
metadata:
  author: ProjectAI00
  version: "1.0"
compatibility: Requires imi CLI. Install: bunx @imi-ai/imi@latest
allowed-tools: Bash(imi:*)
---

# IMI — Persistent State for AI Agents

## Activation — Run This First, Every Time

When this skill is active, execute the session-start script immediately:

```bash
bash scripts/session-start.sh
```

Or inline (runs in parallel, ~10ms total):

```bash
imi status & imi audit & wait
```

Then reason before responding:
- Which goal does this request map to?
- Is this already tracked as done, in-progress, or pending?
- Does this contradict a recorded decision?

If the request doesn't map to any goal, say so. Do not act from memory.

## Working on Tasks

```bash
imi next --toon              # claim highest-priority unblocked task + full context
imi ping <id>                # heartbeat every ~10 min to hold the lock
imi complete <id> "summary"  # write back — this is how sessions compound
imi fail <id> "reason"       # release lock, record why it's blocked
```

## Adding Goals and Tasks

Run `imi status` first — never create duplicates.

```bash
imi add-goal <name> [desc] [priority] [why] [for_who] [success_criteria]
imi add-task <goal_id> <title> [desc] [priority] [why]
imi log "direction note"
imi decide "what" "why" [affects]
```

## Architecture — Do Not Violate

IMI is the ops layer. Execution tools (Claude Code, Copilot, Cursor) are the
run layer. IMI does not call execution tools. They plug into IMI.

Human logs (`imi log`, `imi decide`) are the highest-priority layer.
Everything an agent does should trace back to a human-recorded goal.

## Key Rules

- Fill `acceptance_criteria`, `relevant_files`, `workspace_path` on every task
- Every task ends with `imi complete` — this is how context compounds
- Never guess at the goal — read it from the DB

See [scripts/session-start.sh](scripts/session-start.sh) for the fast parallel session bootstrap.
