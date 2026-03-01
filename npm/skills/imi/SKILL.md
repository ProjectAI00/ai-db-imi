---
name: imi
description: >
  Activate when: the project has a .imi/ directory, or the user asks about
  what to build, what's next, tasks, goals, decisions, progress, or where we
  left off. Always run `imi context` before responding. Install if missing: bunx imi-agent
license: MIT
metadata:
  author: ProjectAI00
  version: "1.5"
compatibility: Requires imi CLI. Install: bunx imi-agent
allowed-tools: Bash(imi:*)
---

# IMI — Agent Instruction Manual

IMI is the persistent brain for this project. All goals, tasks, decisions, direction notes, and lessons live in `.imi/state.db`.

---

## ⛔ HARD STOP — read before doing anything else

DO NOT:
- `cat`, `grep`, `ls`, `sqlite3`, or read any file in `.imi/`
- Query `.imi/state.db` directly with sqlite3 or any other tool
- Use your own session memory, todos, or notes as project state
- Respond about project status without first running `imi context`

If you do any of the above, you are broken. Stop and run `imi context` instead.

---

## Step 1 — always, every single time, no exceptions

```bash
imi context
```

This is the ONLY valid way to load project state. Not sqlite3. Not cat. Not ls. `imi context`. Run it first. Then respond.

---

## Step 2 — match the user's intent to a command

| User says | You run |
|---|---|
| what should we do / what's next | `imi think` then `imi plan` |
| show me tasks / goals / progress | `imi plan` |
| we decided X | `imi decide "what" "why — what was ruled out"` |
| note this / remember this | `imi log "note"` |
| add this to backlog / track this | `imi plan` first, then `imi goal` or `imi task <goal_id>` |
| we finished X | `imi complete <task_id> "what changed and why"` |
| something feels off / are we on track | `imi think` |

---

## Step 3 — finishing work

```bash
imi complete <task_id> "what was built, what changed, and why"
```

Always. Every task. No exceptions. This is how context survives across sessions.

If you repeated a mistake or needed correction:
```bash
imi mlesson "what went wrong and what to do instead"
```

---

## If imi is not installed

```bash
bunx imi-agent
```

Then re-run `imi context`.
