# IMI CLI

> The persistent brain for AI agents. A SQLite DB + bash CLI that any agent can read from and write to.

## The Problem

AI agents are stateless. Every session starts from zero — no memory of what was decided, learned, or shipped. Founders re-explain context every time.

## The Solution

IMI stores goals, tasks, decisions, and memories in a local SQLite DB. Any agent (Claude Code, GitHub Copilot CLI, Cursor, Codex) reads `imi context`, then `imi plan`, and can execute with `imi run` — no briefing required.

## Install

### Option 1 — CLI binary (standalone)

```bash
bunx @imi-ai/imi@latest
```

That's it. Downloads the binary, runs `imi init` in your project folder.

Or via curl:

```bash
curl -fsSL https://aibyimi.com/install | bash
```

Make sure `~/.local/bin` is in your `$PATH`:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

### Option 2 — Plugin (Claude Code + GitHub Copilot CLI)

Install the IMI plugin directly into your agent CLI. The plugin injects the
session contract automatically — agents know to call `imi` before responding,
without manual briefing.

**Claude Code:**
```bash
/plugin marketplace add ProjectAI00/ai-db-imi
/plugin install imi
```

**GitHub Copilot CLI:**
```
/plugin marketplace add ProjectAI00/ai-db-imi
/plugin install imi
```

**Or install the skill manually** (works in Claude Code, Copilot CLI, Cursor — anywhere that reads `~/.copilot/skills` or `~/.claude/skills`):
```bash
npx skills add ProjectAI00/ai-db-imi@imi
```

## Usage

```bash
imi context                           # Human intent first: direction, decisions, current focus
imi plan                              # Goals/tasks/progress plan
imi run <task_id>                     # Execute one task with runtime writeback
imi check                             # Verification snapshot (done work needing review)
imi help                              # Command reference

# Advanced (agent/runtime)
imi next --agent <name>               # Atomically claim next task
imi complete <id> "what you did"      # Mark done + store completion memory
imi fail <id> "why it failed"         # Release task back to queue with failure context
imi goal "name" --why "why now"       # Create a goal
imi task <goal_id> "title" --why "..." # Add a task
imi log "direction note"              # Log strategic direction
imi decide "what" "why"               # Log a decision
imi orchestrate <goal_id> --workers 8 -- <cmd ...>  # Parallel worker loop
```

## The Loop

```
1. You define direction     →  imi context
2. You shape the plan       →  imi plan
3. Agent claims a task      →  imi next --agent claude
4. Agent executes           →  (Claude Code / Copilot / Cursor does the work)
5. Runtime writes back      →  imi wrap <task_id> -- <agent command ...>
                               (auto checkpoints + complete/fail on exit)
6. Next agent picks up      →  imi context + imi plan  ← sees prior context automatically
```

Each cycle compounds. Agents get smarter about your project over time. Works across sessions, machines, and team members.

## Integrations

IMI is the state layer. These tools plug into the execution layer beneath it — IMI doesn't call them, agents choose when to use them.

### Hankweave — task execution & checkpointing

Use Hankweave for long or complex multi-step tasks where you want rollback and checkpointing:

```bash
./imi run <task_id>        # generates hank.json from task context
bunx hankweave hank.json   # execute it
```

Hankweave handles HOW work gets done. IMI handles WHAT was decided and WHAT was learned.

### Entire — session audit & rewind

Use Entire for session replay and audit:

```bash
entire enable --agent claude-code   # hook into your agent sessions
entire rewind                       # replay what happened in a past session
entire explain                      # summarise a session in plain English
```

Entire records what happened. IMI remembers what matters. Forward state + backward audit.

## Stack

- **Rust** — single binary, zero runtime dependencies, ~5ms per command
- **SQLite** — portable, zero-config, project-local (`.imi/state.db`)
- **Hankweave** — optional execution/checkpointing layer
- **Entire** — optional session audit/rewind layer
- **Works with**: Claude Code, GitHub Copilot CLI, Cursor, Codex, any CLI agent

## Agent Prompts

Drop these as system prompts to give any agent full IMI literacy:

| File | Use when |
|------|----------|
| `prompts/plan-mode.md` | Agent is decomposing a goal into tasks |
| `prompts/execute-mode.md` | Agent is executing a task |
| `prompts/ops-mode.md` | Conversational ops / status / decisions |

## Multi-Agent Support

Multiple agents can work in parallel — each claims a different task atomically:

```bash
imi next --agent engineer-a --toon   # Agent A claims task 1
imi next --agent engineer-b --toon   # Agent B claims task 2 (different task)
imi next --agent engineer-c --toon   # Agent C claims task 3
```

If a task is abandoned, IMI auto-releases it after 30 minutes. The next agent picks it up with full failure context.
