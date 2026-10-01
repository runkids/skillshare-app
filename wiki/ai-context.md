# AI Context Router

This page is the in-repo manual for keeping the agent docs routed and true. It works without any skill installed. Load it with `python3 scripts/ai-context.py ai-context`.

## Layers

1. **Kernel:** root `AGENTS.md`, loaded by every agent on every task. It holds only rules that every task needs, plus the one-line triggers that say which topic to load.
2. **Scoped instructions:** optional nested `AGENTS.md` files for one package or app. Every one of them must be loaded by some topic.
3. **Topics:** `docs/ai-context.json` maps each topic to whole files or exact headings. `scripts/ai-context.py <topic>` prints only those sections.
4. **History:** `wiki/history/`, holding milestone logs in their original language. They are never loaded by default and are indexed in `wiki/README.md`.

`docs/ai-context.json` is the single source of truth. The tables in `AGENTS.md` and `wiki/README.md` mirror it for discovery.

## When docs must change

Update docs **in the same change** that makes them stale:

| You did this | Update this |
|---|---|
| Changed behaviour a topic describes | That topic's page |
| Found a doc that contradicts the code | Fix the doc, because the code wins; mention it in your handoff |
| Learned a lesson the hard way | A one-line kernel rule plus the procedure in its topic |
| Finished a milestone | Add `wiki/history/<milestone>.md` and one index row in `wiki/README.md` |
| Added a new kind of recurring task | Add a new topic: page, JSON entry, and both tables |
| Added a nested `AGENTS.md` | Map it in some topic |

Leave volatile state out of the docs: progress counts, who is doing what this week, today's blocker. Those belong in the tracker.

## Adding or changing a topic

1. Put the content in `wiki/<page>.md`. A topic should be a recognizable kind of work, not a single ticket or file.
2. Map it in `docs/ai-context.json`. Reference exact headings for large pages.
3. Add a row to the topic tables in `AGENTS.md` and `wiki/README.md`.
4. Run `python3 scripts/ai-context.py check`, then render the topic and read it as an agent would.

## What check enforces

- Every path stays inside the repo and exists, and every heading matches exactly once.
- `AGENTS.md` and every topic stay under their byte budgets. Over budget means split the topic. Do not raise the cap.
- Every topic is listed in `AGENTS.md`.
- **No orphans:**
  - every `wiki/` page (history excluded) is loaded by some topic;
  - every history file is indexed in `wiki/README.md`;
  - every nested `AGENTS.md` is loaded by some topic.

  A deliberate exception goes under `"unrouted"` in the JSON, with a reason.

`check` runs in CI, so drift fails the build instead of rotting quietly.

## Skills

Skills are workflow adapters. They may call `ai-context.py`, run checks and shorten steps, but no rule may live only in a skill. With no skill installed, this page and the kernel must still be enough.
