---
description: Start implementing a spec
---
Begin implementation of a spec.

1. If $ARGUMENTS contains a spec ID, use it. Otherwise call `list_specs` with `project_dir` "." to find specs ready for implementation
2. Call `get_spec` to read the full spec
3. Call `advance_spec` to move to the "implement" phase (this auto-creates a git branch via the workflow action)
4. Read the spec's acceptance criteria and technical notes carefully
5. Create an implementation plan based on the spec requirements
6. Begin implementing, following the spec requirements
7. Commit work with messages prefixed with `[spec:{id}]`
8. When implementation is complete, ask if the user wants to advance to "verify" phase
