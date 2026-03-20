---
description: Create a new spec using Skillshare App
---
Use the skillshare MCP to create a new spec.

1. Call `list_schemas` with `project_dir` set to the current working directory to show available schemas
2. Ask the user which schema to use if not specified in $ARGUMENTS
3. Call `create_spec` with the chosen schema and title
4. Call `get_spec` with the returned ID to show the full spec
5. Ask if the user wants you to draft the body content
6. If yes, read the schema sections and write appropriate content, then call `update_spec` to save
