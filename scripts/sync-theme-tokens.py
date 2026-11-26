#!/usr/bin/env python3
"""Copy the dashboard's theme tokens into src/theme-tokens.css.

The app's own windows (title bar, settings, onboarding) use the same tokens as the skillshare
CLI's web UI, so they match the dashboard in the iframe. Run this whenever the CLI changes them:

    python3 scripts/sync-theme-tokens.py ../skillshare/ui/src/components.css

It keeps every top-level rule before `@layer components` whose selector names a style
(`data-theme`), plus the @media blocks made of such rules, and drops selectors for the CLI's theme
editor preview (`.ss-mini`), which the app does not have. The result replaces the text between the
BEGIN and END markers in src/theme-tokens.css.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

TARGET = Path(__file__).resolve().parent.parent / "src" / "theme-tokens.css"
BEGIN = "/* BEGIN tokens from skillshare ui/src/components.css (scripts/sync-theme-tokens.py) */"
END = "/* END tokens */"


def blocks(css: str):
    """Yield (prelude, body) for each top-level block, skipping comments."""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    i = 0
    while True:
        open_at = css.find("{", i)
        if open_at < 0:
            return
        depth, j = 1, open_at + 1
        while depth:
            depth += {"{": 1, "}": -1}.get(css[j], 0)
            j += 1
        yield css[i:open_at].strip(), css[open_at + 1 : j - 1]
        i = j


def keep_selectors(prelude: str) -> str | None:
    selectors = [s.strip() for s in prelude.split(",")]
    kept = [s for s in selectors if "data-theme" in s and ".ss-mini" not in s]
    return ",\n".join(kept) if kept else None


def rule(prelude: str, body: str, indent: str = "") -> str | None:
    selector = keep_selectors(prelude)
    if not selector:
        return None
    decls = [d.strip() for d in body.split(";") if d.strip()]
    if not all(d.startswith("--") for d in decls):
        return None
    lines = [indent + "  " + re.sub(r"\s+", " ", d) + ";" for d in decls]
    head = indent + selector.replace("\n", "\n" + indent) + " {"
    return "\n".join([head, *lines, indent + "}"])


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: sync-theme-tokens.py <path to skillshare ui/src/components.css>")
    source = Path(sys.argv[1]).read_text()
    head = source.split("@layer components", 1)[0]
    out = []
    for prelude, body in blocks(head):
        if prelude.startswith("@media"):
            inner = [r for p, b in blocks(body) if (r := rule(p, b, "  "))]
            if inner:
                out.append(f"{prelude} {{\n" + "\n".join(inner) + "\n}")
        elif r := rule(prelude, body):
            out.append(r)
    if not out:
        sys.exit("no theme token rules found; has components.css changed shape?")
    target = TARGET.read_text()
    if BEGIN not in target or END not in target:
        sys.exit(f"{TARGET} is missing the BEGIN/END markers")
    before, rest = target.split(BEGIN, 1)
    after = rest.split(END, 1)[1]
    TARGET.write_text(before + BEGIN + "\n" + "\n".join(out) + "\n" + END + after)
    print(f"wrote {len(out)} rules to {TARGET}")


if __name__ == "__main__":
    main()
