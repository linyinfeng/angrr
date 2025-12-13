# angrr - Auto Nix GC Root Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects that haven't been accessed for a long time won't be automatically removed, preventing old store paths from being garbage collected.

This tool deletes such temporary GC roots based on the **modification time** of their symbolic link targets. Combined with the direnv module that automatically touches the GC roots in the direnv layout directory before loading `.envrc`, the tool can precisely remove direnv GC roots that haven't been **accessed** for a long time.

Except for temporary GC roots create by direnv or `nix build`, this tool can also manage profile-based GC roots (starting from version `0.2.0`).

⚠️**Note**: Direnv integration was added in version `0.1.2`, but the version didn’t make it into the `nixos-25.11` channel — currently it’s only available in `nixos-unstable`.

## Usage

Please refer to the man page of this project.

```console
$ man 1 angrr          # for command usage
$ angrr --help         # or see command line help
$ man 5 angrr          # for configuration file format
$ angrr example-config # extract example configuration file
```

**How to test**: Use the `--dry-run` option to test the changes without actually deleting anything.

## NixOS Module Usage

A NixOS module example:

```nix
{ ... }:
{
  services.angrr = {
    enable = true;
    config = {
      temporary-root-policies = {
        direnv = {
          path-regex = "/\\.direnv/";
          period = "14d";
        };
        result = {
          path-regex = "/result[^/]*$";
          period = "3d";
        };
      };
      profile-policies = {
        system = {
          profile-paths = [ "/nix/var/nix/profiles/system" ];
          keep-since = "14d";
          keep-latest-n = 5;
          keep-booted-system = true;
          keep-current-system = true;
        };
        user = {
          enable = false;
          profile-paths = [
            "~/.local/state/nix/profiles/profile"
            "/nix/var/nix/profiles/per-user/root/profile"
          ];
          keep-since = "1d";
          keep-latest-n = 1;
          keep-booted-system = false;
          keep-current-system = false;
        };
      };
    };
  };
  nix.gc.automatic = true;
  programs.direnv.enable = true;
}
```

## Flake Usage

The tool is available in nixpkgs. But if you want to use the latest version, you can use this flake directly.
An overlay `overlays.default` and a NixOS module `nixosModules.angrr` are provided. Run `nix flake show` to see all available outputs.

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
