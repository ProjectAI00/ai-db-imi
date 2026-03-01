---
name: imi
description: >
  Activate when "imi" appears, when .imi/ exists, or when a prompt is about
  project goals/tasks/priorities/decisions/progress ("what next", "where we
  left off", "add to backlog", "done", "we decided", "scrap feature"). Run
  `imi context` before routing. If `imi` is missing, tell user to run:
  `bunx @imi-ai/imi@latest`. IMI is source of truth; chat memory is not.
license: MIT
metadata:
  author: ProjectAI00
  version: "1.1"
compatibility: Requires imi CLI. Install: bunx @imi-ai/imi@latest
allowed-tools: Bash(imi:*)
---

# IMI — Persistent State for AI Agents

## Required First Step

```bash
imi context
```

Then route the prompt using the table below.

If `imi` is not in PATH, instruct:

```bash
bunx @imi-ai/imi@latest
```

## Prompt → Command Routing

Use explicit command mapping even when user never says "imi":

| Human intent pattern | Command(s) |
|---|---|
| Resume work ("keep working on auth") | `imi context && imi next --toon` |
| Where we left off ("where did we leave off") | `imi context` |
| Choose work now ("what should we work on today") | `imi context && imi next --toon` |
| Record decision ("we decided postgres not mysql") | `imi decide "Use Postgres instead of MySQL" "Team decision"` |
| Add backlog item ("add retry logic to backlog") | `imi context` then `imi add-task <goal_id> "<title>" --why "<reason>"` |
| Record non-negotiable rule ("do NOT store raw card numbers") | `imi decide "<rule>" "<why>"` (optional: `imi log "<note>"`) |
| Mark completion ("stripe integration is done") | `imi context` then `imi complete <task_id> "<summary>"` |
| Cancel feature ("scrap email notifications") | `imi context` then `imi decide "Cancel <feature>" "<why>"` then `imi delete <task_or_goal_id>` |
| Net-new initiative ("build dashboard for usage metrics") | `imi add-goal "<name>" "<description>"` then `imi add-task ...` |
| Repeated agent mistake ("keeps forgetting token expiry") | `imi decide "<guardrail>" "<why>"` (or `imi log` if just observation) |
| Tentative direction ("we should probably rethink onboarding") | `imi log "<direction note>"`, escalate to `imi add-goal` when confirmed |

## Task Lifecycle

```bash
imi next --toon              # claim highest-priority unblocked task + full context
imi ping <id>                # heartbeat every ~10 min to hold the lock
imi complete <id> "summary"  # write back — this is how sessions compound
imi fail <id> "reason"       # release lock, record why it's blocked
```

## Goal/Task Creation

```bash
imi add-goal <name> [desc] [priority] [why] [for_who] [success_criteria]
imi add-task <goal_id> <title> [desc] [priority] [why]
imi log "direction note"
imi decide "what" "why" [affects]
```

## Important Clarifications

- If user asks for generic coding help with no project-state intent (e.g. "write a debounce function"), do the coding task directly and skip IMI routing.
- For "scrap/cancel/kill", always record a decision before deleting stale goals/tasks.
- Prefer `imi decide` for durable rules; use `imi log` for tentative direction.
- For commands needing IDs (`complete`, `delete`, `add-task`), run `imi context` first and use IDs shown there.
