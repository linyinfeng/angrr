# angrr - Auto Nix GC Roots Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects not accessed for a long time won't be automatically removed, leading to many old store paths can not being garbage collected.

This tool simply deletes auto GC roots based on the **modification time(last modified date)** of their symbolic link target.

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

## Direnv integration

Some direnv environments upgrade very rarely. GC roots of such environments will be deleted and recreated frequently even when you are actively using them. Since the tool deletes auto GC roots based on the modification time of their symbolic link target.

To solve this issue, an `angrr touch` command is provided. The command recursively touches every link to store in the given directory.

```console
$  angrr touch .direnv
Touch ".direnv/flake-profile-a5d5b61aa8a61b7d9d765e1daf971a9a578f1cfa"
Touch ".direnv/flake-inputs/8n2y13ilw5vdc058bxsd0xn7bzjpp6m3-source"
Touch ".direnv/flake-inputs/b0my5vy8pzfzjqrr3g58j0w6md9jf3ch-source"
Touch ".direnv/flake-inputs/q67xj3l4kdqqkkpr8ajpcqj7vybrqkqg-source"
Touch ".direnv/flake-inputs/8ly89qdcjh6pb5xamvq4vrzqnifwshn3-source"
Touch ".direnv/flake-inputs/xaccbji1vx054s1r52939z11yfalkjbj-source"
Touch ".direnv/flake-inputs/niayq5b53f1zcz63j2xghghjbya12hpf-source"
```

A direnv library is provided to easy integrate `angrr touch` to direnv. By default, if you are using the NixOS module of `direnv` with `services.angrr.enable = true`, `angrr touch` will automatically be run for the direnv layout directory before loading `.enrvc`.

```console
$ direnv allow
direnv: using angrr
direnv: angrr: touch GC roots /home/yinfeng/Source/angrr/.direnv
direnv: loading ~/Source/angrr/.envrc
direnv: using flake
direnv: nix-direnv: Renewed cache
...
```

This behavior can be turned off by settings `programs.direnv.angrr.autoUse = false`, you can still manually add `use angrr` to explicitly trigger `angrr touch`. If you want to disable direnv integration completely, set `programs.direnv.angrr.enable = false`.
