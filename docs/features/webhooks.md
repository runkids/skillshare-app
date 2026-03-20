# Webhooks

Automate actions with incoming and outgoing webhooks.

## Overview

Skillshare App supports two types of webhooks:

- **Outgoing**: Send notifications when events occur
- **Incoming**: Trigger workflows from external services

<p align="center">
  <img src="../screenshots/webhook-setting.png" width="900" alt="Webhooks" />
</p>

<!-- TODO: Add diagram showing webhook flow -->

## Outgoing Webhooks

### What are Outgoing Webhooks?

Send HTTP requests when Skillshare App events happen:

- Workflow completes
- Script finishes
- Deploy succeeds or fails

### Creating an Outgoing Webhook

1. Open a workflow
2. Click **Settings** → **Webhooks**
3. Click **Add Outgoing Webhook**
4. Configure:
   - URL to send to
   - Events to trigger on
   - Payload template
5. Save

<!-- TODO: Add screenshot of outgoing webhook config -->

### Payload Template

Customize the JSON payload sent:

```json
{
  "event": "${event_type}",
  "workflow": "${workflow_name}",
  "status": "${status}",
  "timestamp": "${timestamp}",
  "duration": "${duration_ms}",
  "project": "${project_name}"
}
```

### Available Variables

| Variable | Description |
|----------|-------------|
| `${event_type}` | Type of event |
| `${workflow_name}` | Workflow name |
| `${workflow_id}` | Workflow ID |
| `${status}` | success/failed |
| `${timestamp}` | ISO timestamp |
| `${duration_ms}` | Duration in milliseconds |
| `${project_name}` | Project name |
| `${project_path}` | Project directory |

### Testing Webhooks

1. Click **Test** on a webhook
2. Skillshare App sends a test payload
3. See the response status

## Incoming Webhooks

### What are Incoming Webhooks?

URLs that trigger Skillshare App actions when called:

- Start a workflow
- Run a script
- Trigger a deploy

### Creating an Incoming Webhook

1. Open a workflow
2. Click **Settings** → **Triggers**
3. Click **Add Webhook Trigger**
4. Skillshare App generates a unique URL
5. Copy the URL

<!-- TODO: Add screenshot of incoming webhook URL -->

### Webhook URL Format

```
http://localhost:{port}/webhook/{token}
```

### Authenticating Requests

Incoming webhooks use token authentication:

- Token is part of the URL
- Keep the URL secret
- Regenerate token if compromised

### Starting the Webhook Server

1. Go to **Settings** → **Webhooks**
2. Enable **Incoming Webhook Server**
3. Set the port (default: 7235)
4. Click **Start**

## Use Cases

### CI/CD Integration

Trigger Skillshare App workflows from CI:

**GitHub Actions:**
```yaml
- name: Trigger Skillshare App
  run: |
    curl -X POST https://your-webhook-url
```

**GitLab CI:**
```yaml
trigger_skillshare-app:
  script:
    - curl -X POST $SKILLSHARE_WEBHOOK_URL
```

### Slack Notifications

Send workflow results to Slack:

1. Create a Slack Incoming Webhook
2. Add as outgoing webhook in Skillshare App
3. Customize the payload for Slack format

**Slack Payload:**
```json
{
  "text": "Workflow '${workflow_name}' ${status}",
  "attachments": [{
    "color": "${status == 'success' ? 'good' : 'danger'}",
    "fields": [
      {"title": "Project", "value": "${project_name}", "short": true},
      {"title": "Duration", "value": "${duration_ms}ms", "short": true}
    ]
  }]
}
```

### Discord Notifications

Similar to Slack, send to Discord webhooks:

**Discord Payload:**
```json
{
  "content": "Workflow completed!",
  "embeds": [{
    "title": "${workflow_name}",
    "description": "Status: ${status}",
    "color": 5763719
  }]
}
```

### Remote Triggers

Start workflows from anywhere:

- Mobile shortcuts
- Smart home integrations
- Other applications

## Webhook Events

### Workflow Events

| Event | Description |
|-------|-------------|
| `workflow.started` | Workflow execution began |
| `workflow.completed` | Workflow finished successfully |
| `workflow.failed` | Workflow failed |
| `workflow.cancelled` | Workflow was cancelled |

### Script Events

| Event | Description |
|-------|-------------|
| `script.started` | Script execution began |
| `script.completed` | Script finished successfully |
| `script.failed` | Script exited with error |

### Deploy Events

| Event | Description |
|-------|-------------|
| `deploy.started` | Deployment began |
| `deploy.completed` | Deployment successful |
| `deploy.failed` | Deployment failed |

## Webhook Logs

View webhook activity:

1. Go to **Settings** → **Webhooks** → **Logs**
2. See:
   - Timestamp
   - Direction (in/out)
   - URL
   - Status code
   - Response time

<!-- TODO: Add screenshot of webhook logs -->

## Security

### Best Practices

1. **Use HTTPS**: For outgoing webhooks, prefer HTTPS URLs
2. **Keep tokens secret**: Don't share webhook URLs publicly
3. **Regenerate tokens**: If a URL is compromised
4. **Validate payloads**: For incoming webhooks, validate the source

### Regenerating Tokens

If a webhook URL is compromised:

1. Open the webhook settings
2. Click **Regenerate Token**
3. Update all integrations with the new URL

## Tips

1. **Test before relying**: Always test webhooks work before depending on them
2. **Check logs**: Review logs to debug issues
3. **Use templates**: Customize payloads for each service
4. **Set timeouts**: Configure reasonable timeout values
5. **Handle failures**: Plan for webhook delivery failures
