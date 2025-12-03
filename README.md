# angrr - Auto Nix GC Roots Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects that haven't been accessed for a long time won't be automatically removed, preventing old store paths from being garbage collected.

This tool deletes auto GC roots based on the **modification time** of their symbolic link targets. Combined with the direnv module that automatically touches the GC roots in the direnv layout directory before loading `.envrc`, the tool can precisely remove direnv GC roots that haven't been **accessed** for a long time.

By default, `angrr` monitors paths matching the regex `/\.direnv/|/result.*$`. Use the `--path-regex <REGEX>` option to customize this behavior.

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

Use the `--dry-run` option to test the changes without actually deleting anything.
Use the `--help` option to see all available options.

For the syntax of `--period <PERIOD>`, please refer to [the documentation of humantime::parse_duration](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html).

## Flake Usage

An overlay `overlays.default` and a NixOS module `nixosModules.angrr` are provided. Run `nix flake show` to see all available outputs.

A NixOS module example:

```nix
{ ... }:
{
  nix.gc.automatic = true;
  services.angrr = {
    enable = true;
    period = "2weeks";
    extraArgs = [
      ...
    ];
  };
}
```

This configuration automatically triggers angrr before `nix-gc.service` with a retention period of 2 weeks. The `--owned-only=false` option is passed by default so the service manages auto GC roots for all users.

Read [nixos/module.nix](./nixos/module.nix) for more information.

## Direnv integration

Some direnv environments upgrade very rarely. GC roots of such environments will be deleted and recreated frequently even when you are actively using them, since the tool deletes auto GC roots based on the modification time of their symbolic link targets.

The `angrr touch` command recursively updates the modification time of every symlink to the Nix store in the given directory.

```console
$  angrr touch .direnv
Touch ".direnv/flake-profile-a5d5b61aa8a61b7d9d765e1daf971a9a578f1cfa"
Touch ".direnv/flake-inputs/8n2y13ilw5vdc058bxsd0xn7bzjpp6m3-source"
...
```

A direnv library is provided to easily integrate `angrr touch` with direnv. By default, if you enable the NixOS module with both `services.angrr.enable = true` and `programs.direnv.enable = true`, `angrr touch` automatically runs for the direnv layout directory before loading `.envrc`.

```console
$ direnv allow
direnv: using angrr
direnv: angrr: touch GC roots /home/yinfeng/Source/angrr/.direnv
direnv: loading ~/Source/angrr/.envrc
direnv: using flake
direnv: nix-direnv: Renewed cache
...
```

You can disable this behavior by setting `programs.direnv.angrr.autoUse = false`. You can still manually add `use angrr` to your `.envrc` to explicitly trigger `angrr touch`. To disable direnv integration completely, set `programs.direnv.angrr.enable = false`.
