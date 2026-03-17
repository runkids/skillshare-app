---
description: Check status of all specs in the project
---
Show the current status of all specs.

1. Call `list_specs` with `project_dir` "." to get all specs
2. For each spec, call `get_workflow_status` to get the current phase and gate statuses
3. Display a summary table:
   - Spec title | Schema | Phase | Gate Status | Last Updated
4. Highlight any specs with failing gates or paused autopilot
5. Call `get_agent_runs` to check for any running AI agents
6. If there are running agents, show their status (spec, phase, duration)
