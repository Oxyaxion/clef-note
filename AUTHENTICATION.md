# Authentication

| Client | Mechanism |
|--------|-----------|
| Web UI — password mode | Password → session token (localStorage, 30-day TTL) |
| Web UI — OIDC mode | Authorization Code + PKCE via external provider |
| CLI (`scripts/cn`) | `CN_KEY` env var = `api_key` from config |

Authentication is **global across all partitions** — one login gives access to every partition. Sessions expire after 30 days or on sign out.

**OIDC mode** — add an `[oidc]` section to `clef-note.toml` (see [Configuration](README.md#configuration)). By default both the OIDC button and the password form are shown, so you can keep a fallback. Set `disable_password_login = true` to show the OIDC button only.

Any provider that exposes a standard OIDC discovery endpoint (`/.well-known/openid-configuration`) should work.

## Authelia setup

Add a client entry to your Authelia configuration:

```yaml
identity_providers:
  oidc:
    clients:
      - client_id: 'clef-note'
        client_name: 'Clef Note'
        client_secret: '$pbkdf2-sha512$...'   # hash with: authelia crypto hash generate pbkdf2 --variant sha512
        public: false
        authorization_policy: 'one_factor'
        require_pkce: true
        pkce_challenge_method: 'S256'
        token_endpoint_auth_method: 'client_secret_basic'   # required — clef-note uses Basic auth by default
        redirect_uris:
          - 'https://notes.example.com/auth/oidc/callback'
        scopes:
          - 'openid'
          - 'profile'
          - 'email'
        grant_types:
          - 'authorization_code'
        response_types:
          - 'code'
        consent_mode: implicit
```

Then in `clef-note.toml`:

```toml
[oidc]
issuer_url    = "https://auth.example.com"
client_id     = "clef-note"
client_secret = "your-plain-text-secret"   # the un-hashed value used above
redirect_uri  = "https://notes.example.com/auth/oidc/callback"
allowed_email = "user@example.com"
provider_name = "Authelia"
disable_password_login = true
```

> **Note:** `token_endpoint_auth_method: 'client_secret_basic'` is required. Authelia defaults to `client_secret_post` for new clients but clef-note (and most OIDC libraries) use `client_secret_basic`, which is the method recommended by RFC 6749.
