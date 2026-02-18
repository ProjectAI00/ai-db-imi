# IMI CLI

> The persistent brain for AI agents. A SQLite DB + bash CLI that any agent can read from and write to.

## The Problem

AI agents are stateless. Every session starts from zero — no memory of what was decided, learned, or shipped. Founders re-explain context every time.

## The Solution

IMI stores goals, tasks, decisions, and memories in a local SQLite DB. Any agent (Claude Code, GitHub Copilot CLI, Cursor, Codex) calls `./imi next --toon` and gets everything it needs to start working — no briefing required.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/ProjectAI00/ai-db-imi/main/imi -o ~/.local/bin/imi
chmod +x ~/.local/bin/imi
imi init   # run inside your project folder
```

Or drop `imi` directly in your project root and call `./imi`.

Make sure `~/.local/bin` is in your `$PATH`:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

## Usage

```bash
imi status                            # Dashboard — goals, tasks, memories
imi context --toon                    # Full context: decisions, direction, WIP
imi next --toon                       # Atomically claim next task (agent-optimized)
imi next --agent <name> --toon        # Claim as a named agent
imi complete <id> "what you did"      # Mark done + store completion memory
imi fail <id> "why it failed"         # Release task back to queue with failure context
imi memory add <goal_id> <key> "val"  # Store a persistent insight
imi decide "what" "why"               # Log an architectural decision
imi log "direction note"              # Log a strategic direction note
imi add-goal "name" "why"             # Create a new goal
imi add-task <goal_id> "title" "desc" # Add a task to a goal
imi ping <task_id>                    # Heartbeat — keep a task alive
```

## The Loop

```
1. You define a goal        →  imi add-goal "Ship auth system" "users need to log in"
2. Agent claims a task      →  imi next --agent claude --toon
3. Agent executes           →  (Claude Code / Copilot / Cursor does the work)
4. Agent writes back        →  imi complete <id> "implemented JWT with RS256"
                               imi memory add <goal> jwt_rotation "rotate every 90d"
5. Next agent picks up      →  imi next --toon  ← sees all prior context automatically
```

Each cycle compounds. Agents get smarter about your project over time. Works across sessions, machines, and team members.

## Integrations

IMI is the state layer. These tools plug into the execution layer beneath it — IMI doesn't call them, agents choose when to use them.

### Hankweave — task execution & checkpointing

Use Hankweave for long or complex multi-step tasks where you want rollback and checkpointing:

```bash
./imi run <task_id>   # generates hank.json from task context and fires hankweave
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
