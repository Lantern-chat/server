embed-worker
============
Standalone and Cloudflare-worker microservice for fetching oEmbed/Embed data from URLs. Using an edge function for this avoids both overhead on the primary server and tracking of primary server by the sites we want embedding information from.

## Shared Configuration (CF Worker and Standalone)

### `CAMO_SIGNING_KEY`
128-bit signing key encoded as a hexidecimal string.

## Standalone Configuration

### `EMBEDW_BIND_ADDRESS`
Sets the bind address for this microservice.

Example `.env`:

```ini
CAMO_SIGNING_KEY = "59d273a2641327d005b255bb7dc89a9f" # Example key
EMBEDW_BIND_ADDRESS = "127.0.0.1:8766"
```