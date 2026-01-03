---
description: Write session handoff summary for learning continuity
allowed-tools:
  - Bash(git status*)
  - Bash(git diff*)
  - Read
  - Write
---

# Handoff Command

Generate a concise session handoff document to maintain learning continuity.

## Steps

1. Check current git status (porcelain format for parsing):
   ```bash
   git status --porcelain
   ```

2. Get diff statistics:
   ```bash
   git diff --stat
   ```

3. Read current learning log if exists:
   ```bash
   # Check dev/learning/ for today's file (YYYY_MM_DD.md)
   ```

4. Write handoff to `dev/learning/HANDOFF.md` with the following structure:

```markdown
# Session Handoff - YYYY-MM-DD HH:MM

## Changes Made

[Concise bullet list of what changed - files, features, refactorings]

## Decisions & Assumptions

[Key technical decisions made during this session]
[Assumptions that might need revisiting]

## Next Steps

1. [Most immediate next task]
2. [Second priority task]
3. [Third priority task]

## Open Questions / Risks

- [Question 1: What needs clarification?]
- [Risk 1: What might cause issues?]

## Learning Notes

[Brief notes on what was learned this session]
[Patterns discovered or understood]
```

## Important Notes

- **DO NOT implement new features** - This is a learning repository
- Focus on **what** changed and **why** decisions were made
- Keep it concise (under 200 words for Changes Made section)
- Highlight learning opportunities discovered during the session
- Mark tasks that are ready for the learner to implement
