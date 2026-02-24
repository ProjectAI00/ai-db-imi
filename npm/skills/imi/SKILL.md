---
name: imi
description: >
  Use this skill whenever the user mentions "imi" or "IMI", or when starting
  any coding session in a project that uses IMI for persistent agent state.
  IMI is a persistent ops layer — SQLite + CLI — that stores goals, tasks, and
  memory across all agent sessions. When active, you must run imi commands
  before doing anything else. The DB is the source of truth, not your context.
license: MIT
metadata:
  author: ProjectAI00
  version: "1.0"
compatibility: Requires imi CLI installed. Install with: npx @imi-ai/imi or brew/cargo from github.com/ProjectAI00/ai-db-imi
allowed-tools: Bash(imi:*)
---

# IMI — Persistent State Engine for AI Agents

IMI solves the stateless agent problem. Every session starts knowing what to
build, why, and what was already learned. You read from it before responding.
You write back to it when done. This is non-negotiable.

## Mandatory Session Start — No Exceptions

Before you respond to anything, before you write a single line of code:

```bash
imi status    # Goals + task state across all sessions
imi audit     # Catch tasks marked done that weren't verified
```

Then reason from what you read:
- What goal does this request map to?
- Is this work already tracked (done, in-progress, or pending)?
- Does this contradict a recorded decision?

If you can't map the user's request to a goal in the DB, say so before acting.
The DB is the source of truth. Your context window is not.

## Claiming and Completing Work

```bash
imi next --toon          # Atomically claim highest-priority task + get full context
imi ping <id>            # Heartbeat every ~10 min to keep task locked
imi complete <id> "summary"   # Mark done — this is how context compounds
imi fail <id> "reason"        # Release lock, record why it's blocked
```

## Adding Goals and Tasks

Check first — `imi status` — before adding anything. Don't create duplicates.

```bash
imi add-goal <name> [desc] [priority] [why] [for_who] [success_criteria]
imi add-task <goal_id> <title> [desc] [priority] [why]
imi log "insight or decision note"
imi decide "what" "why" [affects]
```

## Architecture (Do Not Violate)

```
IMI              → ops layer    (goals, tasks, memory — persists across sessions)
Execution tools  → run layer    (Claude Code, Copilot, Cursor, Codex — HOW work gets done)
```

IMI does not call execution tools. Execution tools plug into IMI. Never couple
IMI to a specific agent. That creates lock-in.

## Key Principles

- `acceptance_criteria`, `relevant_files`, `workspace_path`, `tools` — always fill these on tasks
- Every task ends with `imi complete` — this is how sessions compound
- Thin tasks = agents guess. Rich tasks = agents deliver.
- Human decisions and logs (`imi log`, `imi decide`) are the most important layer. Everything else follows.
