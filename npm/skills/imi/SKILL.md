---
name: imi
description: >
  Activate when: the project has a .imi/ directory, or the prompt is about
  what to build, tasks, goals, decisions, progress, what's next, where we
  left off, or anything related to project direction. Run `imi context` before
  responding. Check `imi help` to understand which command fits the situation.
  If imi is not installed: bunx imi-agent
license: MIT
metadata:
  author: ProjectAI00
  version: "1.3"
compatibility: Requires imi CLI. Install: bunx imi-agent
allowed-tools: Bash(imi:*)
---

# IMI — MANDATORY USAGE RULES

**NEVER read `.imi/` files directly. NEVER use cat, grep, or any file tool on `.imi/` files. ALWAYS use `imi` CLI commands.**

## Always run first
```bash
imi context
```

This is the ONLY way to load project state. Do not substitute with file reads.

## Available commands — use these, nothing else
```bash
imi context     # RUN FIRST EVERY SESSION — what's being built, active tasks, recent decisions
imi think       # reason over everything: are we building the right thing?
imi plan        # full goal and task list with status
imi next        # single next task to work on
imi check       # verification state of tasks

imi goal        # create a new goal (run imi plan first to avoid duplicates)
imi task        # add a task, always attach to a goal_id from imi context
imi complete    # mark task done + store summary as persistent memory (NEVER skip)
imi decide      # record a firm decision with what was ruled out and why
imi log         # capture a direction, instinct, or concern not yet a decision
imi mlesson     # store a lesson when agent repeated a mistake or needed correction
```

## Rules
1. **Start every session with `imi context`** — no exceptions
2. **Never read `.imi/state.db` or any `.imi/` file directly** — use CLI commands
3. **Never use your own memory or session todos** — imi is the source of truth
4. When finishing work: `imi complete <id> "what changed and why"`
5. When user asks "what should we do" or "what's next": run `imi plan` then `imi next`
