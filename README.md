Lantern
=======

Lantern is an upcoming realtime communications platform

## Environment variables

Easiest way to set environment variables is via a `.env` file in the working directory.

Example `.env` file:
```ini
# Backblaze B2 app ID and key
B2_APP=8941f1b62c8d266432c4a9317
B2_KEY=XE2qB2JZR0QXbU8j8PbHIUhXRsvQLNb
# database strings
DB_STR=postgresql://postgres:password@localhost:5432
MIGRATIONS="./sql/migrations" # Migrations path, used to initialize/update database
# MFA encryption key (64 hex digits exact)
MFA_KEY=44fdf0178a056a9c036b650a20524c18a22d624cae2263605e4004e9124502a3
# File encryption key (64 hex digits exact)
FS_KEY=177413173be53067be07bfd967c772db40febb53b27c7ccac94744535df200ab
DATA_DIR="./data" # Location that files will be stored
# Snowflake encryption key (used to obscure IDs in a reversable way) (32 hex digit exacts)
SF_KEY=66908925cc6b22855aca27c8995ca4c3
# 128-512 -bit key used for computing bot token HMACs (min 32 hex digits, max 128 hex digits)
BT_KEY=5f38e06b42428527d49db9513b251651
# Bind address
LANTERN_BIND="localhost:8080"
# TLS Support (not enabled yet)
KEY_PATH="/etc/letsencrypt/live/"
CERT_PATH="/etc/letsencrypt/live/"
# hCaptcha keys
HCAPTCHA_SECRET=0x0000000000000000000000000000000000000000
HCAPTCHA_SITEKEY=10000000-ffff-ffff-ffff-000000000001
```

# LICENSE

All files, projects, and crates within this monorepo, with the exception of external git submodules or otherwise specified within specific files or directories, are licensed under the [PolyForm Non-Commercial License](https://polyformproject.org/wp-content/uploads/2020/05/PolyForm-Noncommercial-1.0.0.txt).