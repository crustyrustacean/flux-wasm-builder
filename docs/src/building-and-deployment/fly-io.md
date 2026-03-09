# Fly.io

A `fly.toml` configuration file is included in every project scaffolded by `wasm-drydock init`. This provides zero-configuration deployment to Fly.io.

## Prerequisites

- [flyctl](https://fly.io/docs/hands-on/install-flyctl/) installed and authenticated

## Quick Deploy

The scaffolded project includes everything needed for deployment:

```bash
# Register the app with Fly (one-time setup)
fly launch --no-deploy

# Deploy
fly deploy
```

The Docker build runs on your local machine. The resulting image is pushed to Fly's registry and deployed.

## Generated `fly.toml`

The scaffolded `fly.toml` is pre-configured:

```toml
app = "your-app-name"    # Uses your project name
primary_region = "yyz"   # Toronto (change as needed)

[build]

[env]
  APP_ENVIRONMENT = "production"
  APP_APPLICATION__HOST = "0.0.0.0"

[http_service]
  internal_port = 3001
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

[[vm]]
  size = "shared-cpu-1x"
  memory = "256mb"
```

## Customization

### Change the primary region

Edit `fly.toml` and change `primary_region` to your preferred region. Common options:
- `yyz` — Toronto
- `sea` — Seattle
- `lax` — Los Angeles
- `fra` — Frankfurt
- `sin` — Singapore

See all regions with `fly regions list`.

### Scale the VM

Edit the `[[vm]]` section in `fly.toml`:

```toml
[[vm]]
  size = "shared-cpu-2x"  # Larger CPU
  memory = "512mb"        # More memory
```

## Custom domain

Add your domain in the Fly dashboard under **Certificates**, then point your DNS at Fly's servers. Fly provisions the TLS certificate automatically.

## Important: bind address

Fly's proxy routes external traffic to your app's internal port. Your app must bind to `0.0.0.0` rather than `127.0.0.1`. The `APP_APPLICATION__HOST = "0.0.0.0"` environment variable in `fly.toml` handles this.

## Skipping deployment files

If you don't need Fly.io deployment, use the `--no-deploy` flag when creating your project:

```bash
wasm-drydock init my-app --no-deploy
```
