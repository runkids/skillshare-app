# Security Audit

Visual npm audit with vulnerability details and one-click fixes.

## Overview

Skillshare App integrates npm audit to help you identify and fix security vulnerabilities in your dependencies.

<p align="center">
  <img src="../screenshots/security.png" width="900" alt="Security audit dashboard" />
</p>

<p align="center">
  <img src="../screenshots/security-analysis.png" width="900" alt="AI security analysis" />
</p>

<!-- TODO: Add gif of running a security scan and reviewing results. -->

## Running a Scan

### Manual Scan

1. Select a project
2. Open the **Security** tab
3. Click **Scan Now**

Skillshare App runs `npm audit` and displays the results.

<!-- TODO: Add gif of running a security scan -->

### Automatic Reminders

Skillshare App can remind you to scan regularly:

1. Go to **Settings** → **Security**
2. Enable **Scan Reminders**
3. Set the frequency (daily, weekly, monthly)

## Understanding Results

### Severity Levels

Vulnerabilities are categorized by severity:

| Level | Color | Description |
|-------|-------|-------------|
| **Critical** | Red | Immediate action required |
| **High** | Orange | Should be fixed soon |
| **Moderate** | Yellow | Fix when convenient |
| **Low** | Blue | Minimal risk |
| **Info** | Gray | Informational only |

<!-- TODO: Add screenshot of severity badges -->

### Vulnerability Card

Each vulnerability shows:

- **Package name**: The vulnerable package
- **Severity**: Critical, High, Moderate, Low
- **Title**: Brief description
- **Path**: Dependency chain to this package
- **Fix available**: Whether a patch exists

## Vulnerability Details

Click on a vulnerability to see full details:

### Overview
- CVE identifier (if available)
- CWE classification
- CVSS score
- Affected versions

### Description
Detailed explanation of the vulnerability and its potential impact.

### Recommendation
How to fix the issue, usually by upgrading to a patched version.

### References
Links to:
- CVE database entry
- GitHub advisory
- Package changelog

<!-- TODO: Add screenshot of vulnerability detail dialog -->

## Fixing Vulnerabilities

### One-Click Fix

For vulnerabilities with available fixes:

1. Click **Fix** on the vulnerability card
2. Skillshare App runs the appropriate command:
   - `npm audit fix` for safe fixes
   - Shows manual steps for breaking changes

<!-- TODO: Add gif of one-click fix -->

### Manual Fix

For complex cases:

1. Review the recommended fix version
2. Update your `package.json` manually
3. Run `npm install`
4. Re-scan to verify the fix

### Breaking Changes

Some fixes may introduce breaking changes. Skillshare App warns you when:

- The fix requires a major version bump
- The fix may affect other dependencies
- Manual testing is recommended

## Direct vs. Transitive

### Direct Dependencies

Packages listed in your `package.json`. You control these directly.

### Transitive Dependencies

Packages installed as dependencies of your dependencies. Fixing these may require:

- Upgrading the direct dependency
- Waiting for the maintainer to fix
- Using `overrides` in `package.json`

<!-- TODO: Add diagram showing direct vs transitive -->

## Scan History

View past scans:

1. Click **History** in the Security tab
2. See the last 10 scans with:
   - Timestamp
   - Total vulnerabilities found
   - Breakdown by severity

Track your progress in reducing vulnerabilities over time.

<!-- TODO: Add screenshot of scan history -->

## Monorepo Support

For monorepos, Skillshare App scans each workspace:

1. Click **Scan All Workspaces**
2. Results are grouped by package
3. Filter by workspace name

<!-- TODO: Add screenshot of monorepo security view -->

## Filtering Results

### By Severity

Filter to show only specific severity levels:

- Show only Critical and High
- Hide Low and Info
- Focus on what matters most

### By Package

Search for vulnerabilities in specific packages.

### By Fix Status

- **Fixable**: Patches available
- **No fix**: Awaiting upstream fix

## Export Report

Generate a security report:

1. Click **Export Report**
2. Choose format:
   - JSON (for CI/CD)
   - Markdown (for documentation)
   - CSV (for spreadsheets)

## Tips

1. **Scan regularly**: Run scans weekly at minimum
2. **Fix critical first**: Prioritize by severity
3. **Update dependencies**: Many vulnerabilities are fixed by updating
4. **Check before deploy**: Run a scan before every production deploy
5. **Review transitive deps**: Sometimes you need to change direct deps to fix transitive issues

## Troubleshooting

### Scan Fails

- Ensure `package-lock.json` exists
- Try running `npm install` first
- Check for network issues

### False Positives

Some vulnerabilities may not affect your usage:

1. Check the vulnerability details
2. Assess if your code uses the affected functionality
3. Consider if the risk is acceptable

### Can't Fix Vulnerability

If no fix is available:

1. Open an issue with the package maintainer
2. Consider alternatives to the vulnerable package
3. Implement workarounds if possible

## Lockfile Validation

Skillshare App includes supply chain security validation for lockfiles. This feature detects potential security issues before they become problems.

### Configuring Validation

1. Go to **Settings** → **Security** → **Lockfile Validation**
2. Enable/disable validation globally
3. Choose strictness level:
   - **Permissive**: Only critical issues
   - **Moderate**: Balanced detection (recommended)
   - **Strict**: Maximum protection

### Validation Rules

| Rule | Description |
|------|-------------|
| **Insecure Protocol** | Detects packages resolved via insecure protocols (git://, http://) |
| **Unexpected Registry** | Flags packages from non-whitelisted registries |
| **Manifest Mismatch** | Detects when lockfile doesn't match package.json |
| **Blocked Package** | Alerts when a blocked package is detected |
| **Missing Integrity** | Flags packages without integrity hashes |
| **Typosquatting Detection** | Identifies potential typosquatting attempts |

### Registry Whitelist

Manage allowed registries:

1. Go to **Settings** → **Security** → **Lockfile Validation**
2. Add trusted registries (e.g., `https://registry.npmjs.org`)
3. Remove untrusted registries

### Blocked Packages

Maintain a blocklist of packages:

1. Add package names to block
2. Provide a reason for blocking
3. Snapshots will flag these packages automatically

### Typosquatting Detection

Skillshare App detects three types of typosquatting:

- **Name similarity**: Levenshtein distance analysis against popular packages
- **Scope confusion**: Detects `@scope/pkg` vs `scope-pkg` patterns
- **Homoglyph attacks**: Identifies lookalike Unicode characters

### Validation Insights

Validation issues appear as security insights in:

- Time Machine snapshots
- Security tab overview
- Project dashboard

Each insight shows severity (critical, high, medium, low, info) and recommended action.

## Security Audit Log

Skillshare App maintains a comprehensive audit log of security-relevant events across the application.

### Accessing the Audit Log

1. Go to **Settings** → **Security** → **Security Audit**
2. View event timeline with filtering options
3. Export logs for compliance or analysis

### Event Types

| Event Type | Description |
|------------|-------------|
| **Webhook Trigger** | External webhook requests and their outcomes |
| **Authentication** | Login attempts, HMAC signature verification |
| **Tool Execution** | AI assistant tool calls and results |
| **Security Alert** | Rate limiting, suspicious activity |
| **Data Access** | Sensitive data access events |
| **Configuration** | Security-related setting changes |

### Actor Types

Events are attributed to different actor types:

- **User**: Manual user actions
- **AI Assistant**: Actions performed by the AI assistant
- **Webhook**: External webhook requests
- **System**: Automated system operations

### Filtering Events

Filter the audit log by:

- **Time Range**: Last 24 hours, 7 days, or 30 days
- **Event Type**: Filter by specific event categories
- **Actor Type**: Filter by who performed the action
- **Outcome**: Success, Failure, or Denied

### Event Details

Click on an event to see:

- **Event ID**: Unique identifier for the event
- **Resource**: Type and name of the affected resource
- **Outcome Reason**: Why the event succeeded or failed
- **Actor Details**: Session ID, source IP
- **Additional Details**: JSON payload with extra context
- **Timestamp**: Exact time of the event

### Exporting Logs

Export audit logs for compliance or external analysis:

1. Click **Export** in the Security Audit panel
2. Logs are exported as JSON
3. Includes all filtered events

### Retention Policy

- Audit logs are automatically cleaned up after 90 days
- Cleanup runs automatically on each new event insert
- Export logs before retention period if needed for compliance
