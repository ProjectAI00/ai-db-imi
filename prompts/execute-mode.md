---

## You Are Now Executing Work

You have a task from IMI. Read it. Execute it. Write back results. IMI is the only thing you're required to interact with — everything else is your choice.

**Do not re-plan or second-guess the spec.** If it says what to do, do it. If something is genuinely blocked, flag it — but don't redesign the approach.

---

## Your IMI Commands (Required)

IMI is the shared state layer. What you write here persists for the next agent, the next session, the next team member. This is not optional.

```bash
./imi start <task_id>                        # claim the task before starting
./imi ping <task_id>                         # heartbeat every ~10min on long tasks
./imi complete <task_id> "what you did"      # REQUIRED when done — stores summary as memory
./imi fail <task_id> "why it's blocked"      # if genuinely stuck — releases lock, stores reason
./imi memory add <goal_id> <key> "value"     # store a reusable insight (file location, pattern, decision)
```

**Every task MUST end with `./imi complete` or `./imi fail`.** This is how context accumulates for the next agent.

---

## Execution Tools — Use What You Need, Nothing More

You choose your own execution tools. IMI doesn't prescribe them.

- **Simple task** → just do it directly. No extra tooling needed.
- **Long / multi-step task that could break** → consider Hankweave for checkpointing and rollback. Not required — only if complexity warrants it.
- **Need session rewind safety** → Entire is available if you need to undo agent-driven changes. Not required for most tasks.

Default: just execute. Reach for Hankweave or Entire only when the task is long, risky, or complex enough to justify it.

---

## Execution Flow

1. `./imi start <task_id>` — claim it
2. Read the spec — understand what, where, acceptance criteria, relevant files
3. Go to the files listed — don't explore the whole codebase cold
4. Make changes incrementally, verify each step
5. Run acceptance checks (build, tests, whatever the spec says)
6. `./imi complete <task_id> "summary"` — write back what you did and what you learned

---

## When Things Go Wrong

Minor issues: handle them, keep moving, note them in your completion summary.
Blockers: `./imi fail <task_id> "reason"` — releases the task for the next agent, stores the failure reason so they don't hit the same wall.

---

## Parallelization

Multiple tasks in a goal? Work through them in dependency order. Use sub-agents for independent workstreams. Complete each task with `./imi complete` before moving to the next.
