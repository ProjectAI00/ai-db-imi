---
name: imi
description: >
  Activate when: the project has a .imi/ directory, or the prompt is about
  what to build, tasks, goals, decisions, progress, what's next, where we
  left off, or anything related to project direction. Run `imi context` before
  responding. Check `imi help` to understand which command fits the situation.
  If imi is not installed: bunx @imi-ai/imi@latest
license: MIT
metadata:
  author: ProjectAI00
  version: "1.2"
compatibility: Requires imi CLI. Install: bunx @imi-ai/imi@latest
allowed-tools: Bash(imi:*)
---

# IMI

Run first:
```bash
imi context
```

Check available commands and when to use each:
```bash
imi help
```

IMI is the persistent state layer for this project. Goals, tasks, decisions, and direction all live in `.imi/state.db`. Use IMI commands for anything related to project state — not your own session memory or todo tools.

When finishing work: `imi complete <id> "what changed and why"`
