# IMI CLI

> The persistent brain for AI agents. A SQLite DB + bash CLI that any agent can read from and write to.

## The Problem

AI agents are stateless. Every session starts from zero — no memory of what was decided, learned, or shipped. Founders re-explain context every time.

## The Solution

IMI stores goals, tasks, decisions, and memories in a local SQLite DB. Any agent (Claude Code, Copilot CLI, Cursor, Codex) calls `./imi next --toon` and gets everything it needs to start working — no briefing required.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/ProjectAI00/ai-db-imi/main/imi -o ~/.local/bin/imi
chmod +x ~/.local/bin/imi
imi init   # in your project folder
```

Or just drop `imi` in your project and call `./imi`.

## Usage

```bash
imi status                          # Dashboard
imi context                         # What matters right now
imi next --toon                     # Claim next task (agent-optimized output)
imi complete <id> "what you did"    # Mark done + store memory
imi run <task_id> [model]           # Fire hankweave to execute a task
```

## The Loop

```
1. You define a goal      → imi add-goal
2. Agent claims a task    → imi next --toon
3. Agent executes         → imi run (via hankweave)
4. Agent writes back      → imi complete "summary" + imi memory add
5. Next agent picks up    → has full context from step 4
```

Each cycle compounds. Agents get smarter about your project over time.

## Stack

- Plain bash — no dependencies, works everywhere
- SQLite — portable, no server
- Hankweave — execution runtime with checkpointing/rollback (`imi run`)
- Works with: Claude Code, GitHub Copilot CLI, Cursor, Codex, any CLI agent

## Prompts

`prompts/plan-mode.md` — inject as system prompt when an agent is creating goals/tasks. Guides it to discover files, acceptance criteria, workspace path.

`prompts/execute-mode.md` — injected automatically by `imi run` into the hankweave execution.
