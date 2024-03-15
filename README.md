# angrr - Auto Nix GC Roots Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects not accessed for a long time won't be automatically removed, leading to many old store paths can not being garbage collected.

This tool simply deletes auto GC roots based on the modified time of the target.

## Usage

```bash
angrr run --period 7d
```

Use the `--dry-run` option to try.
Use the `--help` option for more options.

For the syntax of `--period <PERIOD>`, please refer to [the documentation of humantime::parse_duration](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html).

## Flake Usage

An overlay `overlays.default` and a NixOS module `nixosModules.angrr` is provided.
