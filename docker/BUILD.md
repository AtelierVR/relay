# Docker Build Guide

This document describes the different Dockerfile variants and their use cases.

## Dockerfile Variants

### `Dockerfile` (Production)
**Location:** Root directory  
**Use case:** Production deployments

**Features:**
- Multi-stage build for minimal image size (~15MB)
- Alpine Linux base (security and size)
- Dependency caching for faster rebuilds
- Full optimization (LTO, single codegen unit)
- Stripped binaries

**Build:**
```bash
docker build -t nox-relay:latest .
# or
npm run compose
```

**Size:** ~15MB

---

### `docker/Dockerfile.dev` (Development)
**Use case:** Local development and debugging

**Features:**
- Debug symbols included
- No aggressive optimizations
- Faster build times
- Easier debugging with gdb/lldb

**Build:**
```bash
docker build -f docker/Dockerfile.dev -t nox-relay:dev .
# or
npm run compose:dev
```

**Size:** ~40MB

---

### `docker/Dockerfile.fast` (Quick Testing)
**Use case:** Rapid iteration and testing

**Features:**
- Debian base (faster package installs)
- Minimal build optimizations
- Quick builds for testing changes
- Release mode but faster compilation

**Build:**
```bash
docker build -f docker/Dockerfile.fast -t nox-relay:fast .
# or
npm run compose:fast
```

**Size:** ~60MB

---

### `docker/Dockerfile.sccache` (Incremental Builds)
**Use case:** CI/CD and repeated builds

**Features:**
- sccache for compilation caching
- Reuses compiled artifacts across builds
- Significantly faster on repeated builds
- Ideal for CI pipelines

**Build:**
```bash
docker build -f docker/Dockerfile.sccache -t nox-relay:cached .
```

**Note:** Requires Docker BuildKit:
```bash
DOCKER_BUILDKIT=1 docker build -f docker/Dockerfile.sccache -t nox-relay:cached .
```

**Size:** ~15MB (runtime), cache stored separately

---

## Build Comparison

| Dockerfile | First Build | Rebuild | Size | Use Case |
|------------|-------------|---------|------|----------|
| Production | ~5 min | ~3 min | 15MB | Deployment |
| Dev | ~3 min | ~2 min | 40MB | Debugging |
| Fast | ~2 min | ~1 min | 60MB | Testing |
| Sccache | ~5 min | ~30s | 15MB | CI/CD |

## Docker Compose

For local development with all services:

```bash
docker-compose up -d
```

This uses the production Dockerfile by default. Edit `docker-compose.yml` to change variants.

## Best Practices

1. **Development:** Use `Dockerfile.dev` for daily work
2. **Testing:** Use `Dockerfile.fast` for quick smoke tests
3. **CI/CD:** Use `Dockerfile.sccache` for faster pipeline builds
4. **Production:** Always use root `Dockerfile` for deployments

## Optimization Tips

### Dependency Caching
All Dockerfiles use layer caching. Ensure `Cargo.toml` and `Cargo.lock` are stable before modifying source code.

### Build Cache
Use BuildKit for better caching:
```bash
export DOCKER_BUILDKIT=1
```

### Multi-platform Builds
```bash
docker buildx build --platform linux/amd64,linux/arm64 -t nox-relay:latest .
```

## Troubleshooting

### Build fails with "out of disk space"
Clean up Docker cache:
```bash
docker system prune -a
```

### Slow builds on first run
First builds compile all dependencies. Subsequent builds reuse cached layers.

### sccache not working
Ensure BuildKit is enabled and cache mount is accessible:
```bash
DOCKER_BUILDKIT=1 docker build -f docker/Dockerfile.sccache .
```
