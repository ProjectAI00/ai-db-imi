# AI Voice Guide for IMI Summaries and Memory

Use this guide when writing `imi complete`, `imi fail`, and `imi memory add` content. It defines what to say and how to say it so future agents can trust and reuse stored context.

## 1) Completion Summary Structure (`imi complete`)

Every completion summary must include four parts:

1. **What was built**
   - Name concrete outputs: files, functions, commands, tests.
   - State the change in past tense.
2. **How the intent was interpreted**
   - State how you read the task and what scope you executed.
   - Call out if you narrowed or expanded scope and why.
3. **Uncertainty or drift**
   - State anything ambiguous, unverifiable, or mismatched vs spec.
   - Be explicit about what was verified vs not verified.
4. **Future agent note**
   - One practical handoff line: what to check first before touching this area again.

**Template:**

```text
Built: <concrete changes with files/functions/tests>.
Interpretation: <how task intent was read and applied>.
Drift/uncertainty: <spec mismatch, risk, or unverifiable point>.
Future-agent note: <first thing to know/check next time>.
```

## 2) Voice Principles

- Write in **direct, past tense**: `Implemented`, `Added`, `Removed`, `Verified`.
- Use **no hedging**: avoid `tried`, `attempted to`, `maybe`, `seems`, `probably`.
- Avoid fluff words like **"successfully"** and **"properly"**.
- Prefer **specific references** over abstractions:
  - Good: `prompts/ai-voice.md`, `src/main.rs:L47`, `fire_analytics()`
  - Bad: `the file`, `some logic`, `the system`
- Keep statements falsifiable: each line should be checkable in code or command output.

## 3) Memory Entries (`imi memory add`)

A memory entry must answer one specific reusable question.

- Start with the question it answers.
- Format: `Question? → exact answer`.
- Include concrete location when relevant (file, function, line, command).
- Store durable facts, constraints, patterns, or locations.
- **Never** use memory entries for status updates or completion notes.

**Pattern:**

```text
Where is <thing>? → <file:function:line or command>
Why does <behavior> happen? → <constraint/decision>
What must stay true in this area? → <rule>
```

## 4) Failure Summaries (`imi fail`)

Every failure summary must include three parts:

1. **What was attempted**
   - Searches, files inspected, commands run, edits attempted.
2. **Exactly where it broke**
   - Concrete blocker location: file/line, missing dependency, failing command, contradictory spec line.
3. **What next agent needs**
   - Clear prerequisite or next step to avoid repeating the same wall.

**Template:**

```text
Attempted: <specific actions and commands>.
Blocked at: <exact failure point and why it blocks completion>.
Next-agent need: <prerequisite decision/fix/input before retry>.
```

## 5) Bad vs Good Examples

### A) Completion summaries (`imi complete`)

**Bad:** `Updated prompt files.`

**Good:** `Built: Added prompts/ai-voice.md with sections for summary structure, voice rules, memory format, failure format, and examples. Interpretation: Treated the task as a companion to prompts/execute-mode.md focused on writing quality, not command usage. Drift/uncertainty: No spec mismatch found; no runtime verification needed because this was documentation-only. Future-agent note: Keep examples concrete with file/function references when expanding this guide.`

**Bad:** `Done. Everything looks good.`

**Good:** `Built: Documented mandatory four-part completion summary format and three-part failure format in prompts/ai-voice.md. Interpretation: Implemented concise policy text under 400 lines with actionable templates. Drift/uncertainty: Did not validate against live task outputs because acceptance was file-content based. Future-agent note: If execute-mode tone changes, update wording here to stay aligned.`

### B) Memory entries (`imi memory add`)

**Bad:** `status → finished this task`

**Good:** `Where are summary-writing rules defined? → prompts/ai-voice.md`

**Bad:** `note → changed some docs`

**Good:** `How should memory entries be formatted? → Start with a question and answer using 'Question? → concrete location/fact' (prompts/ai-voice.md)`

**Bad:** `update → task complete`

**Good:** `What must fail summaries include? → attempted actions, exact blocker location, and next-agent prerequisite (prompts/ai-voice.md)`

### C) Failure summaries (`imi fail`)

**Bad:** `Could not finish, got stuck.`

**Good:** `Attempted: Searched prompts/*.md for prior AI-voice guidance and drafted new rules. Blocked at: task required approval for creating new prompt file in a read-only environment; write command failed with permission denied on prompts/ai-voice.md. Next-agent need: rerun with write access or apply patch in a writable workspace.`

**Bad:** `Build failed.`

**Good:** `Attempted: Ran cargo test after editing src/main.rs to wire summary formatting. Blocked at: tests/db_integration.rs failed because TEST_DATABASE_URL was unset; failure occurs before migration setup. Next-agent need: provide TEST_DATABASE_URL or run only unit tests not requiring DB.`

**Bad:** `Spec seems wrong.`

**Good:** `Attempted: Implemented task using relevant files and acceptance criteria from the task spec. Blocked at: spec references prompts/summary-style.md, but repository only contains prompts/execute-mode.md and prompts/plan-mode.md. Next-agent need: update task spec to the correct file path before retrying.`
