# angrr - Auto Nix GC Roots Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects not accessed for a long time won't be automatically removed, leading to many old store paths can not being garbage collected.

This tool simply deletes auto GC roots based on the modified time of their symbolic link target.

## Usage

- For non-root users:

  The following command deletes all auto GC roots older than 7 days owned by the current user.

  ```bash
  nix run github:linyinfeng/angrr -- run --period 7d
  ```

- Manage GC roots for all users as the root user:

  The following command deletes all auto GC roots older than 7 days owned by all users.

  ```bash
  sudo nix run github:linyinfeng/angrr -- run --period 7d --owned-only=false
  ```

  The following command deletes auto GC root links in `/nix/var/nix/gcroots/auto` instead of the symbolic link target of the roots.

  ```bash
  sudo nix run github:linyinfeng/angrr -- run --period 7d --remove-root
  ```

Use the `--dry-run` option to have a try.
Use the `--help` option for more options.

For the syntax of `--period <PERIOD>`, please refer to [the documentation of humantime::parse_duration](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html).

## Flake Usage

An overlay `overlays.default` and a NixOS module `nixosModules.angrr` is provided. Run `nix flake show` for more outputs.

A NixOS module example:

```nix
{ ... }:
{
  nix.gc.automatic = true;
  services.angrr = {
    enable = true;
    period = "2weeks";
  };
}
```

This configuration will automatically trigger angrr before `nix-gc.service`,
and the retention period is 2 weeks,
the `--owned-only=false` option will be passed by default so that the service manages auto GC roots for all users.

Read [nixos/module.nix](./nixos/module.nix) for more information.
