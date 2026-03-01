---
name: imi
description: >
  Activate when: the project has a .imi/ directory, or the user asks about
  what to build, what's next, tasks, goals, decisions, progress, or where we
  left off. Always run `imi context` before responding. Install if missing: bunx imi-agent
license: MIT
metadata:
  author: ProjectAI00
  version: "1.4"
compatibility: Requires imi CLI. Install: bunx imi-agent
allowed-tools: Bash(imi:*)
---

# IMI — Agent Instruction Manual

IMI is the persistent brain for this project. All goals, tasks, decisions, direction notes, and lessons live in `.imi/state.db`. You must use `imi` CLI commands to read and write it. Never access `.imi/` files directly.

---

## Session start — always do this first

```bash
imi context
```

Run this before every single response. It tells you what's being built, what's active, recent decisions, and direction. This is your ground truth. Do not proceed without it.

---

## During a session — how to behave

**User asks what to work on / what's next:**
```bash
imi think   # reason over the full state, surface what's misaligned or missing
imi plan    # show full goal + task list
```
Then recommend the most valuable next task based on what you read.

**User wants to capture a decision:**
```bash
imi decide "what was decided" "why — what was ruled out, what assumption this rests on"
```

**User mentions something important but not yet a firm decision:**
```bash
imi log "direction, instinct, concern, or thing to revisit"
```

**User says to add something to the backlog / track a task:**
```bash
imi goal "name" "why it matters"          # if it's a new initiative
imi task <goal_id> "what needs to be done"  # if it belongs to an existing goal
```
Always run `imi plan` first to check the goal doesn't already exist.

---

## Finishing work — never skip this

```bash
imi complete <task_id> "what was built, what changed, and why"
```

This marks the task done AND stores a persistent memory so the next session knows what happened. The summary is how context compounds. Always call this when work is done.

**If you (the agent) made a mistake that had to be corrected, or repeated an error:**
```bash
imi mlesson "what went wrong and what to do instead"
```

---

## Rules

1. **Never read `.imi/` files directly** — no cat, grep, or file reads on `.imi/`
2. **`imi context` runs before every response** — no exceptions
3. **`imi complete` runs after every finished task** — no exceptions
4. **Never use session memory or built-in todos** — imi is the only source of truth
5. **Run `imi plan` before creating goals or tasks** — avoid duplicates
6. **If `imi` is not installed:** run `bunx imi-agent` to install it, then retry
