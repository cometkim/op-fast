# Unreleased

Remove db-store from dependencies. To make sure secrets are only stored in OS native keyring, and avoid binary bloating by accidently added turso library.

Binary size is shrunk down by ~16M (24M -> 8M)

# Version 0.1.0 (2026-02-26)

Initial release (alpha)
