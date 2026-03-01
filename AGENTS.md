# IMI Session Contract for Agents

Before any project response or code change, run:

```bash
imi context
```

IMI is the source of truth (goals, tasks, decisions, direction). Do not rely on chat memory.

## Routing Rules (No "imi" Mention Needed)

Map natural-language prompts to IMI commands:

1. **Resume prior work** ("keep working on auth")  
   `imi context && imi next --toon`
2. **See where we left off** ("where did we leave off")  
   `imi context`
3. **Pick today's work** ("what should we work on today")  
   `imi context && imi next --toon`
4. **Record a decision** ("we decided postgres over mysql")  
   `imi decide "Use Postgres instead of MySQL" "Team decision"`
5. **Create backlog task** ("add X to backlog")  
   `imi context` then `imi add-task <goal_id> "<task title>" --why "<reason>"`  
   (If your runtime exposes alias `imi task`, use equivalent arguments.)
6. **Record critical guardrail / must-remember rule** ("never store raw cards")  
   `imi decide "<rule>" "<why>"` (optional supporting note: `imi log "<note>"`)
7. **Mark shipped work done** ("stripe integration is done")  
   `imi context` then `imi complete <task_id> "<summary>"`
8. **Cancel/scrap a feature** ("scrap email notifications")  
   `imi context` then `imi decide "Cancel <feature>" "<why>"` then remove stale item: `imi delete <task_or_goal_id>`
9. **Create net-new initiative** ("build a usage metrics dashboard")  
   `imi add-goal "<name>" "<description>"` then seed first task via `imi add-task ...`
10. **Capture repeated agent mistake** ("keeps forgetting token expiry")  
   `imi decide "<non-negotiable rule>" "<why>"` (or `imi log` if only observational)
11. **Tentative strategic direction** ("we should probably rethink onboarding")  
    `imi log "<direction note>"` (promote to `imi add-goal` once confirmed)

## Non-Project / No-IMI Context Exception

If the prompt is generic and not about this repo state (e.g., "write a debounce function"), do the coding task directly and skip IMI routing.
For commands that need IDs (`imi complete`, `imi delete`, `imi add-task`), use `imi context` first, then pick the exact ID from context output.

## Completion Writeback

When finishing tracked work, always write back with:

```bash
imi complete <id> "what changed and why"
```
