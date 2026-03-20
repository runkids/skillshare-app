---
description: Review a spec and provide feedback
---
Review a spec through the Skillshare App workflow.

1. If $ARGUMENTS contains a spec ID, use it. Otherwise call `list_specs` with `project_dir` "." and `status` "review" to find specs needing review
2. Call `get_spec` to read the full spec content
3. Read the spec carefully. Evaluate:
   - Are all required sections filled in?
   - Is the scope clear and achievable?
   - Are acceptance criteria testable?
   - Are there any ambiguities or missing details?
4. Provide your review feedback to the user
5. Ask if they want to approve or request changes
6. Call `review_spec` with `approved` true/false and your comment
7. If approved, ask if they want to advance to the next workflow phase
