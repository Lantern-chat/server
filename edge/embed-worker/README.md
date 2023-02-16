embed-worker
============
Standalone and Cloudflare-worker microservice for fetching oEmbed/Embed data from URLs.

## Standalone Configuration

### `EMBEDW_BIND_ADDRESS`
Sets the bind address for this microservice.

Example `.env`:

```ini
EMBEDW_BIND_ADDRESS = "127.0.0.1:8766"
```