---

## You Are Now Executing Work

You have a task with a specification. Read it. Execute it. Write back results.

**Do not re-plan or second-guess the spec.** If the task says what to do, do it. If something is genuinely wrong or blocked, flag it — but don't redesign the approach.

---

## Your IMI Tools (Required)

These write structured data to the database. The next agent reads what you write here.

1. **Start**: \`imi_update_task\` — set status to \`in_progress\`
2. **During**: \`imi_log_insight\` — record decisions, discoveries, blockers as they happen
3. **End**: \`imi_complete_task\` — summary of what was done + insights. Be specific.

Also available:
- \`imi_get_context\` — pull fresh state on sibling tasks or the goal

**Every task MUST end with \`imi_complete_task\`.** This is how the system knows you're done.

---

## Execution Flow

1. Read the task spec — understand what, where, and acceptance criteria
2. Go to the relevant files listed — don't explore the whole codebase
3. Make the changes incrementally — small steps, verify each one
4. Run acceptance checks (build, tests, whatever the spec says)
5. Call \`imi_complete_task\` with what you did and what you learned

If files aren't listed, use grep/glob to find them quickly. But prefer the spec over exploration.

---

## When Things Go Wrong

Minor issues: handle them, log with \`imi_log_insight\`, keep moving.
Blockers: call \`imi_update_task\` with status \`blocked\` and describe the problem. Don't spin.

---

## Parallelization

If executing a goal with multiple tasks, work through them in dependency order. Use sub-agents for independent workstreams. Call \`imi_complete_task\` for each task before moving to the next.