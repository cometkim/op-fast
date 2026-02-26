# op-fast: Speed up your 1Password CLI

`op-fast` is a proxy for the [1Password CLI], to make secret access much faster, like instantly ⚡

```
Benchmark 1: op read "op://Employee/Test Note/test"
  Time (mean ± σ):      2.031 s ±  0.032 s    [User: 0.072 s, System: 0.052 s]
  Range (min … max):    1.980 s …  2.062 s    5 runs

Benchmark 2: op-fast read "op://Employee/Test Note/test"
  Time (mean ± σ):      22.6 ms ±   1.7 ms    [User: 4.9 ms, System: 4.1 ms]
  Range (min … max):    21.2 ms …  25.2 ms    5 runs

Summary
  op-fast read "op://Employee/Test Note/test" ran
   90.00 ± 6.91 times faster than op read "op://Employee/Test Note/test"
```

It leverages OS keyrings to cache secrets that have already been fetched. To make it fast re-access without requiring re-authentication or network roundtrips.

## Features

- **Offline access**: Secrets cached in OS keyring (macOS Keychain, Linux keyutils)
- **Configurable TTL**: Set default expiration or per-secret patterns
- **Full CLI compatibility**: Drop-in replacement for `op` command

## Installation

Check out the [releases page](https://github.com/cometkim/op-fast/releases) for pre-built binaries.

You can install it using Homebrew:

```bash
brew install cometkim/tap/op-fast
```

Or from source:

```bash
cargo install op-fast
```

## Usage

Basic usage is same with the original [1Password CLI].

### read

Read a secret reference:

```bash
# Read a password
op-fast read op://app-prod/db/password

# With variable substitution
VAULT=prod op-fast read 'op://$VAULT/db/password'

# Save to file (default mode: 600)
op-fast read -o ./key.pem op://app-prod/ssh/private-key

# Custom file mode
op-fast read -o ./key.pem --file-mode 644 op://app-prod/ssh/private-key
```

### inject

Inject secrets into a config template:

```bash
# From stdin
echo 'password: {{ op://app-prod/db/password }}' | op-fast inject

# From file to file
op-fast inject -i config.yml.tpl -o config.yml

# With environment variables
echo 'db: op://$ENV/db/password' | ENV=prod op-fast inject

# Custom output file mode
op-fast inject -i config.yml.tpl -o config.yml --file-mode 600
```

Supports two secret reference syntaxes:
- `{{ op://vault/item/field }}` (enclosed)
- `op://vault/item/field` (unenclosed)

### run

Pass secrets as environment variables to a process:

```bash
# With environment variable
DB_PASSWORD='op://app-prod/db/password' op-fast run -- printenv DB_PASSWORD
# Output: <concealed by 1Password>

# With env file
echo 'DB_PASSWORD=op://app-dev/db/password' > .env
op-fast run --env-file .env -- printenv DB_PASSWORD

# Switch environments with variables
cat .env
# DB_PASSWORD=op://$APP_ENV/db/password

APP_ENV=prod op-fast run --env-file .env -- printenv DB_PASSWORD

# Show secrets without masking
DB_PASSWORD='op://app-prod/db/password' op-fast run --no-masking -- printenv DB_PASSWORD
```

### store

Manage the `op-fast` store (OS keyring + cache metadata):

```bash
# List all cached secrets
op-fast store list

# Clear a specific secret
op-fast store clear 'op://vault/item/field'

# Clear all cached secrets
op-fast store clear
```

### Passthrough

Any unrecognized command is passed through to the real `op` binary:

```bash
op-fast item list  # => op item list
op-fast vault list # => op vault list
op-fast whoami     # => op whoami
```

You can even add an alias to your shell profile:

```bash
alias op=op-fast
```

## Configuration

Configuration file: `~/.config/op-fast/config.toml`

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
| `OP_FAST_CONFIG` | Custom config file path |
| `OP_FAST_STORE_DIR` | Custom cache directory |
| `OP_FAST_DEFAULT_TTL` | Override default TTL (e.g., `12h`, `1day`) |

### TTL Format

Human-readable duration format:
- `30s` - 30 seconds
- `5m` - 5 minutes
- `1h` - 1 hour
- `1day` - 1 day
- `1w` - 1 week

## How It Works

1. Cache layer: LMDB stores cache metadata (TTL, timestamps), OS keyring stores secret values
4. Batch fetching: Multiple uncached secrets fetched in a single `op inject` call
2. Automatic GC: Expired entries cleaned up with 10% probability on each invocation

## Security

Secrets stored in OS-native keyring (encrypted at rest)

## License

MIT

[1Password CLI]: https://developer.1password.com/docs/cli
