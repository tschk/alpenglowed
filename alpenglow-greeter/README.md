# alpenglow-greeter

Black & white GPUI login UI for [greetd](https://github.com/kennylevinsen/greetd). **Not** part of the `alpenglowed` shell binary.

## Build

```sh
# from alpenglowed repo root
cargo build --release -p alpenglow-greeter
```

Linux graphical images use `../alpenglow/system/backends/appliance/scripts/build-alpenglow-greeter-glibc.sh`.

## greetd

Greeter runs inside cage (one Wayland client):

`/usr/bin/alpenglow-greeter-cage.sh` → cage → `alpenglow-greeter-run.sh` → `alpenglow-greeter-bin`

Config: `../alpenglow/system/backends/appliance/rootfs-overlay/etc/greetd/config.toml`

## Autologin (skip greeter)

greetd still runs; GPUI login is skipped.

```sh
# on image
ln -sf /etc/greetd/config-autologin.toml /etc/greetd/config.toml
```

Or build QEMU image with:

```sh
ALPENGLOW_AUTOLOGIN=1 ./scripts/boot-native.sh --graphical
```

## Env

- `ALPENGLOW_GREETER_USER` — default username field
- `/etc/alpenglow/greeter-default-user` — same, on disk
- `GREETD_SOCK` — default `/run/greetd.sock`