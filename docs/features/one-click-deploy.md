# One-Click Deploy

Deploy your projects to Netlify, Cloudflare Pages, or GitHub Pages with a single click.

## Overview

Skillshare App integrates with popular hosting platforms, enabling:

- One-click deployments
- Instant preview links
- Environment variable management
- Multi-account support

<p align="center">
  <img src="../screenshots/deploy-demo.gif" width="900" alt="One-click deploy demo" />
</p>

<!-- TODO: Add screenshot of Deploy panel (accounts + config + logs). -->

## Supported Platforms

| Platform | Auth Method | Features |
|----------|-------------|----------|
| **Netlify** | OAuth | Full integration |
| **Cloudflare Pages** | API Token | Full integration |
| **GitHub Pages** | GitHub Actions | Workflow generation |

## Connecting Accounts

### Netlify

1. Go to **Settings** → **Deploy Accounts**
2. Click **Add Account** → **Netlify**
3. Click **Connect with Netlify**
4. Authorize Skillshare App in the browser
5. Account is now connected

<!-- TODO: Add screenshot of Netlify OAuth flow -->

### Cloudflare Pages

1. Go to **Settings** → **Deploy Accounts**
2. Click **Add Account** → **Cloudflare**
3. Enter your Cloudflare API Token
4. Click **Verify & Save**

To create an API token:
1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens)
2. Click **Create Token**
3. Use the **Edit Cloudflare Pages** template
4. Copy the token

<!-- TODO: Add screenshot of Cloudflare token dialog -->

### Multiple Accounts

You can connect multiple accounts:
- Multiple Netlify accounts
- Multiple Cloudflare accounts
- Mix of different platforms

Set a default account for each platform in Settings.

## Build Configuration

### Automatic Detection

Skillshare App automatically detects your framework and suggests:

- Build command (e.g., `npm run build`)
- Output directory (e.g., `dist`, `.next`, `build`)
- Node.js version

<!-- TODO: Add screenshot of auto-detected build config -->

### Supported Frameworks

| Framework | Build Command | Output |
|-----------|---------------|--------|
| Vite | `vite build` | `dist` |
| Next.js | `next build` | `.next` |
| Nuxt | `nuxt build` | `.output` |
| Create React App | `react-scripts build` | `build` |
| Remix | `remix build` | `build` |
| Astro | `astro build` | `dist` |

### Custom Configuration

Override detected settings:

1. Open the Deploy panel
2. Click **Edit Configuration**
3. Modify:
   - Build command
   - Output directory
   - Install command
   - Node version

## Environment Variables

### Adding Variables

1. Open **Environment Variables** in the Deploy panel
2. Click **Add Variable**
3. Enter key and value
4. Choose visibility:
   - **Production**: Only in production
   - **Preview**: Only in preview deploys
   - **All**: Both environments

<!-- TODO: Add screenshot of environment variables panel -->

### Secret Variables

For sensitive values:

1. Toggle **Secret** when adding
2. Value is encrypted
3. Never shown in logs or UI after saving

### Importing from `.env`

1. Click **Import from .env**
2. Select your `.env` file
3. Review imported variables
4. Save to deploy configuration

## Deploying

### Manual Deploy

1. Select a project
2. Open the Deploy panel
3. Choose a deploy account
4. Click **Deploy**

<!-- TODO: Add gif of deploy process -->

### Deploy Progress

During deployment, see:
- Current status
- Build logs
- Any errors or warnings

### Preview Links

After successful deployment:
- **Production URL**: Your live site
- **Preview URL**: Unique URL for this deploy

<!-- TODO: Add screenshot of deploy complete with URLs -->

## Deployment History

View past deployments:

1. Open the Deploy panel
2. Click **History**
3. See all deployments with:
   - Timestamp
   - Status (success/failed)
   - Duration
   - Commit info

### Rollback

To rollback to a previous deploy:

1. Find the deployment in history
2. Click **Rollback**
3. Confirm the action

## GitHub Pages

GitHub Pages works differently — Skillshare App generates a GitHub Actions workflow.

### Setup

1. Select **GitHub Pages** as deploy target
2. Click **Generate Workflow**
3. Review the generated `.github/workflows/deploy.yml`
4. Commit and push

### How It Works

The workflow:
1. Triggers on push to main/master
2. Installs dependencies
3. Runs build command
4. Deploys to `gh-pages` branch

<!-- TODO: Add screenshot of generated workflow file -->

## Deploy Backup

### Export Configuration

Backup your deploy settings:

1. Go to **Settings** → **Backup**
2. Click **Export Deploy Config**
3. Save the JSON file

### Import Configuration

Restore from backup:

1. Go to **Settings** → **Backup**
2. Click **Import Deploy Config**
3. Select your backup file
4. Review and confirm

## Tips

1. **Use preview deploys**: Test changes before production
2. **Set up environment variables first**: Avoid failed deploys
3. **Check build logs**: Understand failures quickly
4. **Use multiple accounts**: Separate personal and work projects
5. **Backup configurations**: Save time when setting up new machines

## Troubleshooting

### Build Failed

- Check the build logs for errors
- Verify your build command works locally
- Ensure all environment variables are set

### Deploy Stuck

- Check platform status (Netlify/Cloudflare)
- Cancel and retry the deploy
- Check for large file uploads

### Missing Environment Variables

- Verify variable names match exactly
- Check if variables are set for the correct environment
- Ensure secrets are properly configured
