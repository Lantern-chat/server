camo-worker
============
Standalone and Cloudflare-worker microservice for proxying cryptographically signed URLs. This helps protect users' privacy with untrusted embeds.

## Shared Configuration (CF Worker and Standalone)

### `CAMO_SIGNING_KEY`
128-bit signing key encoded as a hexidecimal string.

## Standalone Configuration

### `CAMO_BIND_ADDRESS`
Sets the bind address for this microservice.

Example `.env`:

```ini
CAMO_SIGNING_KEY = "59d273a2641327d005b255bb7dc89a9f" # Example key
CAMO_BIND_ADDRESS = "127.0.0.1:8765"
```