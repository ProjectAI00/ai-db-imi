# IMI CLI — AI Agent Instructions

This repo IS the IMI CLI. You are working on the tool that agents use to persist goals, tasks, and memory across sessions.

## What IMI Is

IMI is a persistent state engine for AI agents. It's a SQLite DB + bash CLI that any agent (Claude Code, Copilot, Cursor, Codex) can read from and write to. It solves the stateless agent problem: every session starts knowing what to build, why, and what was already learned.

## Every Session — Do This First

```bash
./imi status          # See all goals, tasks, progress
./imi context         # What matters right now
```

## When Working on a Task

```bash
./imi next --toon     # Atomically claim highest-priority task + get full context
./imi ping <id>       # Heartbeat every 10min to keep task locked
./imi complete <id> "what you did"   # Mark done, store summary as memory
./imi fail <id> "why it's blocked"   # Release lock, store failure for next agent
```

## When Adding Goals/Tasks

```bash
./imi add-goal <name> [desc] [priority] [why] [for_who] [success_criteria]
./imi add-task <goal_id> <title> [desc] [priority] [why]
./imi log "insight or direction note"
./imi decide "what" "why" [affects]
```

## Running Tasks with Hankweave

```bash
./imi run <task_id> [model]   # Generates hank.json from task context + fires hankweave
```

This uses `prompts/execute-mode.md` as the system prompt. The agent writes `summary.md` when done.

## Repo Structure

```
imi              → the CLI bash script (source of truth)
prompts/
  plan-mode.md   → system prompt for planning agents (discovers codebase, creates rich tasks)
  execute-mode.md → system prompt for executing agents (injected into imi run)
.imi/
  state.db       → SQLite DB (goals, tasks, memories, decisions)
  runs/          → hankweave execution dirs per task
```

## The Goal

Make the `plan → execute → writeback` loop work so well that each agent session compounds on the last. An agent should be able to run `./imi next --toon` and have everything it needs to start work immediately — no re-briefing, no guessing.

## Key Principles

- **Fields that matter on a task:** `acceptance_criteria`, `relevant_files`, `workspace_path`, `tools` — always fill these when creating tasks
- **Every task ends with `./imi complete`** — this is how context accumulates
- **Thin tasks = agents guess. Rich tasks = agents deliver.**
