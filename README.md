# angrr - Auto Nix GC Root Retention

If you are a heavy user of [nix-direnv](https://github.com/nix-community/nix-direnv), you might find that auto GC roots of projects that haven't been accessed for a long time won't be automatically removed, preventing old store paths from being garbage collected.

This tool helps clean up old Nix GC roots based on configurable policies.

**Features**:

1. Clean profile generations (system, nix-darwin, nix-env, nix profile, home-manager, ...)
2. Clean temporary roots (nix-direnv/nix-build/nix build, ...)
3. **Highly configurable** policies
4. **Set and forget**

⚠️**Note**:

The version in the stable `nixos-25.11` channel is outdated (`v0.1.1`) and lacks some features, such as profile generation cleaning. If you need that version, refer to the [v0.1.1 README](https://github.com/linyinfeng/angrr/tree/v0.1.1).

## Usage

Refer to the man pages or command-line help:

```console
$ man 1 angrr          # command usage
$ angrr --help         # command-line help
$ man 5 angrr          # configuration file format
$ angrr example-config # extract example configuration file
```

How to test: run with `--dry-run` to see what would be deleted without performing any deletions.

## NixOS Module Usage

If you just want a simple setup, enable the NixOS module with preset settings:

```nix
{ ... }:
{
  services.angrr = {
    enable = true;
    period = "7d";
  };
  # angrr.service runs before nix-gc.service by default
  nix.gc.automatic = true;
  programs.direnv.enable = true;
}
```

Or you can define your own policies in the `services.angrr.settings` option:

```nix
{ ... }:
{
  services.angrr = {
    enable = true;
    settings = {
      temporary-root-policies = {
        direnv = {
          path-regex = "/\\.direnv/";
          period = "14d";
        };
        result = {
          path-regex = "/result[^/]*$";
          period = "3d";
        };
        # You can define your own policies
        # ...
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
          enable = false; # Policies can be individually disabled
          profile-paths = [
            # `~` at the beginning will be expanded to the home directory of each discovered user
            "~/.local/state/nix/profiles/profile"
            "/nix/var/nix/profiles/per-user/root/profile"
          ];
          keep-since = "1d";
          keep-latest-n = 1;
        };
        # You can define your own policies
        # ...
      };
    };
  };
  # angrr.service runs before nix-gc.service by default
  nix.gc.automatic = true;
  programs.direnv.enable = true;
}
```

## Flake Usage

The tool is available in nixpkgs. But if you want to use the latest version, you can use this flake directly.
An overlay `overlays.default` and a NixOS module `nixosModules.angrr` are provided. Run `nix flake show` to see all available outputs.

## Direnv integration

The `angrr touch` command recursively updates the modification time of every symlink to the Nix store in the given directory. So that these paths are retained by angrr or any other Nix GC root retention tools.

```console
$  angrr touch .
Touch "./.direnv/flake-profile-a5d5b61aa8a61b7d9d765e1daf971a9a578f1cfa"
Touch "./.direnv/flake-inputs/8n2y13ilw5vdc058bxsd0xn7bzjpp6m3-source"
...
Touch "./result"
...
```

A direnv library is provided to easily integrate `angrr touch` with direnv. By default, if you enable the NixOS module with both `services.angrr.enable = true` and `programs.direnv.enable = true`, `angrr touch --project` automatically runs for the **project root** before loading `.envrc`.

```console
$ direnv allow
direnv: using angrr
direnv: angrr: touch GC roots in "/home/yinfeng/Source/angrr" (took 0.017s)
direnv: loading ~/Source/angrr/.envrc
direnv: using flake
direnv: nix-direnv: Renewed cache
...
```

You can customize the behavior of `angrr touch --project` in configuration file. By default, `angrr touch --project` ignores `.git` to slightly speed up the touch process. You can modify the `touch.project-globs` list to include or exclude other directories as needed. For example, if you do not want `result`s directories under project root to be touched, you can add `!result*` to the list.

You can completely disable the auto touch behavior by setting `programs.direnv.angrr.autoUse = false`. You can still manually add `use angrr` to your `.envrc` to explicitly trigger `angrr touch`. To disable direnv integration completely, set `programs.direnv.angrr.enable = false`.

### Compare to nix-direnv's refresh functionality

After <https://github.com/nix-community/nix-direnv/pull/631>, nix-direnv also adds its own refresh functionality.

Compared to nix-direnv, angrr touches GC roots under the project root, while nix-direnv touches GC roots under the `direnv_layout_dir`(most of the time, `.direnv` in project root).

Nix-direnv's refresh functionality is enough for most use cases. But angrr's may also cover some very special use cases, so I still provide it.
