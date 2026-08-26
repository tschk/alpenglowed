# Alpenglow session contract

Alpenglowed is a Wayland **client of Cage**. Product SKUs are
`potato | desktop | internet`. This binary is `potato` and `desktop`.
`internet` is `sold`. Machine-readable: `alpenglowed --session-contract`.

| SKU | Build | `ALPENGLOWED_ROLE` |
| --- | --- | --- |
| `potato` | `cargo build --release --no-default-features` | `potato` |
| `desktop` | `cargo build --release` | `desktop` |
| `internet` | — | not this binary |

Role: `ALPENGLOWED_ROLE`, else `/run/alpenglow/role`, else `desktop`.
`potatoes` maps to `potato`. Anything else exits 2.

Session start lives in Alpenglow
`system/backends/appliance/scripts/alpenglow-session-start`.
Do not pass `--features compositor`. Potato does not link smithay.
