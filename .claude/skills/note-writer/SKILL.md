---
description: Technical note writing guidelines for kamomo-notes
---

# Note Writer

This skill provides guidelines for creating high-quality technical notes in the kamomo-notes repository.

## Purpose

kamomo-notes is a personal research note repository powered by MkDocs Material. Notes should be:
- **Practical**: Based on real implementation experience
- **Reusable**: General concepts, not tied to specific projects
- **Clear**: Well-structured with examples and explanations
- **Discoverable**: Properly tagged and organized

## File Structure

### Directory Layout

```
docs/notes/posts/
├── 2026/
│   └── 01/
│       └── 11/
│           ├── 2026-01-11-00-topic-one.md
│           ├── 2026-01-11-01-topic-two.md
│           └── 2026-01-11-02-topic-three.md
```

### Naming Convention

**Format**: `YYYY-MM-DD-NN-descriptive-title.md`

- `YYYY-MM-DD`: Publication date
- `NN`: Sequential number (00, 01, 02...) for multiple posts per day
- `descriptive-title`: Kebab-case English title

**Examples**:
- `2026-01-11-00-hexagonal-architecture.md`
- `2026-01-11-01-rust-condvar-and-locks.md`
- `2026-01-11-02-rust-design-patterns.md`

## Frontmatter Format

All notes must include proper frontmatter:

```yaml
---
title: "Descriptive Title: Main Concept and Context"
date: YYYY-MM-DD
tags:
  - primary-topic
  - secondary-topic
  - technology
  - concept
---
```

**Guidelines:**
- **Title**: Clear, descriptive, and specific
- **Date**: Publication date in ISO format
- **Tags**: 3-7 relevant tags, ordered by importance
  - Use lowercase
  - Use hyphens for multi-word tags: `design-patterns`, `object-safety`
  - Categories: Language/Technology, Concept, Specific Topic, Level (optional)

## Document Structure

```markdown
# Title (repeated from frontmatter)

## 概要

Brief overview (2-3 sentences) explaining what this note covers and why it matters.

---

## 1. Main Concept

### Problem Statement
What problem does this solve? Why is it important?

### Solution/Pattern
How to solve it, with code examples

### Key Points
Bullet points highlighting important takeaways

---

## 2. Related Concept
...

---

## Common Mistakes

### Mistake 1: Description
❌ Wrong way (with explanation)
✅ Correct way (with explanation)

---

## Checklist

- [ ] Item 1
- [ ] Item 2
...

---

## Summary

Concise recap with comparison tables or key takeaways

---

## References

- Official documentation
- Related RFCs
- Other resources
```

## Writing Principles

### 1. Stay Project-Agnostic

**❌ Don't:**
```markdown
In Weaver v2, we use InMemoryDeliveryQueue to implement...
```

**✅ Do:**
```markdown
Example: Implementing a delivery queue with Mutex and Condvar
```

**When to mention projects:**
- In the References section: "Based on implementation in Weaver v2"
- If the pattern is highly specific: "This approach was developed for..."

### 2. Start with the Problem

Always explain **why** before **how**:

```markdown
## Problem: async context で Mutex を使うリスク

### ❌ 危険なコード
[code showing the problem]

### なぜ危険か？
[explanation of why it's dangerous]

## Solution: spawn_blocking
[the correct approach]
```

### 3. Provide Clear Examples

**Code examples should be:**
- **Complete**: Runnable or close to it
- **Annotated**: Include comments for key points
- **Contrasted**: Show both ❌ wrong and ✅ correct approaches

```rust
// ❌ 間違い: Immutable borrow で pop しようとする
let queue = queues.get(&ns)?;
if let Some(task_id) = queue.pop_front() {  // エラー！
    return Ok(Some(task_id));
}

// ✅ 正しい: Mutable borrow を使う
if let Some(queue) = queues.get_mut(&ns) {
    if let Some(task_id) = queue.pop_front() {
        return Ok(Some(task_id));
    }
}
```

### 4. Use Visual Aids

Include diagrams, tables, and structured comparisons:

**Comparison Tables:**
```markdown
| 項目 | Option A | Option B |
|------|---------|---------|
| 性能 | 速い | 遅い |
| 使用場所 | ... | ... |
```

**Flowcharts (ASCII or Mermaid):**
```
┌─────────────────────────┐
│ Async Runtime Thread    │
├─────────────────────────┤
│ Task A: lock() → .await │
│ Task B: waiting...      │
└─────────────────────────┘
```

### 5. Include Checklists

Practical checklists help readers apply the knowledge:

```markdown
## Checklist

実装時に確認すべきポイント：

- [ ] Spurious wakeup 対策: ループで条件を毎回チェック
- [ ] `wait_timeout()` の戻り値: 新しい guard を受け取って更新
- [ ] timeout: 残り時間を計算して渡す
```

## Content Categories

### Technical Concepts

**Focus on:**
- Core concepts (Mutex, async/await, trait objects)
- Common patterns (Strategy, Builder, Ports & Adapters)
- Language features (let chains, turbofish syntax)
- Best practices and anti-patterns

**Structure:**
1. Problem/motivation
2. Explanation
3. Examples (✅ and ❌)
4. Checklist or summary

### Implementation Patterns

**Focus on:**
- Reusable code patterns
- Design patterns in Rust
- Architectural approaches

**Structure:**
1. Pattern name and purpose
2. When to use it
3. Implementation example
4. Variations
5. Trade-offs

### Troubleshooting Guides

**Focus on:**
- Common mistakes
- Error messages and their meanings
- Debugging strategies

**Structure:**
1. Problem symptoms
2. Root cause analysis
3. Solutions
4. Prevention strategies

## Language Guidelines

### Bilingual Content

- **Headings**: English or Japanese (be consistent within a document)
- **Code comments**: English (for portability)
- **Explanations**: Japanese (primary audience)
- **Technical terms**: Use original English terms in parentheses

Example:
```markdown
## Spurious Wakeup（偽の起床）

**問題:** Condvar は理由なく起きることがある（OS の実装による）
```

### Tone and Style

- **Clear and direct**: Avoid unnecessary jargon
- **Practical**: Focus on what readers need to know
- **Empathetic**: Acknowledge common difficulties
- **Encouraging**: Frame mistakes as learning opportunities

## Code Guidelines

### Code Blocks

Always specify the language:

````markdown
```rust
// Rust code here
```

```bash
# Shell commands here
```
````

### Annotations

Use comments to highlight key points:

```rust
let (new_guard, result) = condvar.wait_timeout(guard, remaining).unwrap();
guard = new_guard;  // ✅ guard を更新
```

### Complete Examples

Prefer complete, runnable examples:

```rust
// ✅ Complete example
use std::sync::{Arc, Mutex, Condvar};

fn example() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    // ... rest of the example
}
```

## Quality Checklist

Before publishing a note, verify:

### Content Quality
- [ ] Clear problem statement or motivation
- [ ] At least one complete code example
- [ ] Both correct (✅) and incorrect (❌) examples where applicable
- [ ] Explanation of why each approach works/fails
- [ ] Practical checklist or summary

### Structure
- [ ] Proper frontmatter with title, date, tags
- [ ] Clear section headings
- [ ] Summary section at the end
- [ ] References section (if applicable)

### Code Quality
- [ ] All code blocks have language specifiers
- [ ] Code is properly formatted and runnable
- [ ] Key points are annotated with comments
- [ ] Examples are realistic and practical

### Clarity
- [ ] Technical terms are explained or linked
- [ ] Sentences are clear and concise
- [ ] No project-specific details (unless clearly marked)
- [ ] Consistent terminology throughout

## MkDocs Commands

### Development Server
```bash
# Start local development server (accessible on network)
uv run mkdocs serve -a '0.0.0.0:8000'
```

### Build
```bash
# Build static site (outputs to site/ directory)
uv run mkdocs build
```

## Mathematical Content

Mathematical expressions are fully supported:
- Inline math: `\(x^2 + y^2 = z^2\)`
- Display math: `\[ \sum_{i=1}^{n} i = \frac{n(n+1)}{2} \]`
- MathJax handles LaTeX syntax with proper escaping

## References

- **SKILLS.md**: `/home/ochir/study/kamomo-notes/SKILLS.md` - Full writing guidelines with examples
- **CLAUDE.md**: `/home/ochir/study/kamomo-notes/CLAUDE.md` - Project overview and architecture
- MkDocs Material: https://squidfunk.github.io/mkdocs-material/
- Blog plugin: https://squidfunk.github.io/mkdocs-material/plugins/blog/

## Example Notes

See the notes created on 2026-01-11 for examples of well-structured technical notes in `/home/ochir/study/kamomo-notes/docs/notes/posts/2026/01/11/`:

1. **2026-01-11-01-rust-condvar-and-locks.md**: Problem-solution structure with detailed troubleshooting
2. **2026-01-11-02-rust-design-patterns.md**: Pattern catalog with comparisons
3. **2026-01-11-03-rust-object-safety-and-type-system.md**: Conceptual explanation with practical examples
4. **2026-01-11-04-async-rust-spawn-blocking.md**: Problem-focused with clear before/after examples
5. **2026-01-11-05-rust-functional-programming-idioms.md**: Pattern reference with code snippets
