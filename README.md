# op-offline

1Password CLI wrapper for instant access to secrets.

Caches 1Password secret references in the OS keyring with configurable TTL. Provides instant access to previously fetched secrets without requiring re-authentication or network access.

## Features

- **Offline access**: Secrets cached in OS keyring (macOS Keychain, Linux keyutils)
- **Configurable TTL**: Set default expiration or per-secret patterns
- **Full CLI compatibility**: Drop-in replacement for `op read`, `op inject`, `op run`

## Installation

TBD

## Usage

Basic usage is same with the original 1Password CLI. You can refer [offical docs](https://developer.1password.com/docs/cli)

### read

Read a secret reference:

```bash
# Read a password
op-offline read op://app-prod/db/password

# With variable substitution
VAULT=prod op-offline read 'op://$VAULT/db/password'

# Save to file (default mode: 600)
op-offline read -o ./key.pem op://app-prod/ssh/private-key

# Custom file mode
op-offline read -o ./key.pem --file-mode 644 op://app-prod/ssh/private-key
```

### inject

Inject secrets into a config template:

```bash
# From stdin
echo 'password: {{ op://app-prod/db/password }}' | op-offline inject

# From file to file
op-offline inject -i config.yml.tpl -o config.yml

# With environment variables
echo 'db: op://$ENV/db/password' | ENV=prod op-offline inject

# Custom output file mode
op-offline inject -i config.yml.tpl -o config.yml --file-mode 600
```

Supports two secret reference syntaxes:
- `{{ op://vault/item/field }}` (enclosed)
- `op://vault/item/field` (unenclosed)

### run

Pass secrets as environment variables to a process:

```bash
# With environment variable
DB_PASSWORD='op://app-prod/db/password' op-offline run -- printenv DB_PASSWORD
# Output: <concealed by 1Password>

# With env file
echo 'DB_PASSWORD=op://app-dev/db/password' > .env
op-offline run --env-file .env -- printenv DB_PASSWORD

# Switch environments with variables
cat .env
# DB_PASSWORD=op://$APP_ENV/db/password

APP_ENV=prod op-offline run --env-file .env -- printenv DB_PASSWORD

# Show secrets without masking
DB_PASSWORD='op://app-prod/db/password' op-offline run --no-masking -- printenv DB_PASSWORD
```

### store

Manage the `op-offline` store (OS keyring + cache metadata):

```bash
# List all cached secrets
op-offline store list

# Clear a specific secret
op-offline store clear 'op://vault/item/field'

# Clear all cached secrets
op-offline store clear
```

### Passthrough

Any unrecognized command is passed through to the real `op` binary:

```bash
op-offline item list  # => op item list
op-offline vault list # => op vault list
op-offline whoami     # => op whoami
```

## Configuration

Configuration file: `~/.config/op-offline/config.toml`

```toml
# Default TTL for cached secrets (default: 1day)
default_ttl = "1day"

# Per-secret TTL patterns (glob syntax)
[ttl]
"op://prod/*" = "1hour"
"op://dev/*" = "7days"
"op://*/ssh/*" = "30days"
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `OP_OFFLINE_CONFIG` | Custom config file path |
| `OP_OFFLINE_DEFAULT_TTL` | Override default TTL (e.g., `12h`, `1day`) |
| `OP_OFFLINE_STORE_DIR` | Custom cache directory |

### TTL Format

Human-readable duration format:
- `30s` - 30 seconds
- `5m` - 5 minutes
- `1h` - 1 hour
- `1day` - 1 day
- `1w` - 1 week

## How It Works

1. Cache layer: LMDB stores metadata (TTL, timestamps), OS keyring stores secret values
4. Batch fetching: Multiple uncached secrets fetched in a single `op inject` call
2. Automatic GC: Expired entries cleaned up with 10% probability on each invocation

## Security

Secrets stored in OS-native keyring (encrypted at rest)

## License

MIT
