# Fly.io

## Prerequisites

- [flyctl](https://fly.io/docs/hands-on/install-flyctl/) installed and authenticated

## Setup

Register the app with Fly without deploying:

```bash
fly launch --no-deploy
```

## `fly.toml`

```toml
app = "your-app-name"
primary_region = "yyz"

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

## Deploying

```bash
fly deploy
```

The Docker build runs on your local machine. The resulting image is pushed to Fly's registry and deployed.

## Custom domain

Add your domain in the Fly dashboard under **Certificates**, then point your DNS at Fly's servers. Fly provisions the TLS certificate automatically.

## Important: bind address

Fly's proxy routes external traffic to your app's internal port. Your app must bind to `0.0.0.0` rather than `127.0.0.1`. The `APP_APPLICATION__HOST = "0.0.0.0"` environment variable in `fly.toml` handles this.
