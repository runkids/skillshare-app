# Visual Workflow

Build automation flows with a drag-and-drop editor. Chain scripts, trigger other workflows, and automate your development process.

## Overview

Visual Workflow lets you create multi-step automation without writing scripts. Perfect for:

- Build and deploy pipelines
- Testing sequences
- Release processes
- Daily development routines

<p align="center">
  <img src="../screenshots/workflow.png" width="900" alt="Workflow editor" />
</p>

<!-- TODO: Add a short gif of dragging nodes and connecting edges. -->

## Creating a Workflow

### New Workflow

1. Navigate to the **Workflows** tab
2. Click **New Workflow**
3. Give your workflow a name
4. Start adding nodes

### The Canvas

The workflow editor is a visual canvas where you:

- **Drag** nodes from the sidebar
- **Connect** nodes with edges
- **Configure** each node's settings

<!-- TODO: Add gif of dragging and connecting nodes -->

## Node Types

### Script Node

Executes a shell command or npm script.

**Configuration:**
- **Command**: The command to run
- **Working Directory**: Where to run the command (optional)
- **Timeout**: Maximum execution time (optional)

<!-- TODO: Add screenshot of script node configuration -->

### Trigger Workflow Node

Triggers another workflow, enabling nested automation.

**Configuration:**
- **Workflow**: Select the workflow to trigger
- **Wait for completion**: Whether to wait before proceeding

> Note: Skillshare App detects circular references and prevents infinite loops.

## Connecting Nodes

### Creating Connections

1. Click and drag from a node's **output handle** (right side)
2. Drop on another node's **input handle** (left side)
3. The connection is created

### Parallel Execution

Connect multiple nodes to the same output to run them in parallel:

```
        ┌─→ [Test Unit]
[Build]─┼─→ [Test E2E]
        └─→ [Test Integration]
```

### Sequential Execution

Connect nodes in a chain for sequential execution:

```
[Build] → [Test] → [Deploy]
```

<!-- TODO: Add screenshot showing parallel vs sequential execution -->

## Failure Strategies

Configure what happens when a node fails:

### Stop on Failure (Default)

The workflow stops immediately when any node fails.

### Continue on Failure

The workflow continues with remaining nodes even if one fails.

<!-- TODO: Add screenshot of failure strategy selector -->

## Running Workflows

### Manual Trigger

1. Select a workflow from the list
2. Click **Run Workflow**
3. Watch the execution in real-time

### Execution Visualization

During execution:
- **Running nodes**: Animated border
- **Completed nodes**: Green checkmark
- **Failed nodes**: Red X with error details

<!-- TODO: Add gif of workflow execution with node states -->

## Execution History

View past workflow runs:

1. Select a workflow
2. Click **History** in the panel
3. See all past executions with:
   - Timestamp
   - Duration
   - Status (success/failed)
   - Logs for each node

<!-- TODO: Add screenshot of execution history panel -->

## Workflow Templates

### Save as Template

Save frequently used workflows as templates:

1. Create and configure your workflow
2. Click **Save as Template**
3. Give it a name and description
4. The template is now available for future use

### Use a Template

1. Click **New Workflow**
2. Select **From Template**
3. Choose a template
4. Customize as needed

<!-- TODO: Add screenshot of template selection dialog -->

## Environment Variables

Set environment variables for all nodes in a workflow:

1. Open workflow settings
2. Add key-value pairs
3. Variables are available to all script nodes

## Tips

1. **Start simple**: Begin with 2-3 nodes and expand as needed
2. **Use templates**: Save common patterns as templates
3. **Check history**: Review past runs to debug failures
4. **Name nodes clearly**: Use descriptive names for easier debugging
5. **Test incrementally**: Run workflows after adding each node

## Examples

### Build and Deploy

```
[Install Dependencies] → [Build] → [Run Tests] → [Deploy to Staging]
```

### Daily Development

```
[Pull Latest] → [Install] → [Start Dev Server]
```

### Release Pipeline

```
[Run Tests] → [Build] → [Bump Version] → [Create Tag] → [Deploy]
```
