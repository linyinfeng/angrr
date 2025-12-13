{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.angrr;
  direnvCfg = config.programs.direnv.angrr;
  toml = pkgs.formats.toml { };
  configOptions = {
    freeformType = toml.type;
    options = {
      owned_only = lib.mkOption {
        type =
          with lib.types;
          enum [
            "auto"
            "true"
            "false"
          ];
        default = "auto";
        description = ''
          Only monitors owned symbolic link target of GC roots.

          - "auto": behaves like true for normal users, false for root.
          - "true": only monitor GC roots owned by the current user.
          - "false": monitor all GC roots.
        '';
      };
      temporary_root_policies = lib.mkOption {
        type = with lib.types; attrsOf (submodule temporaryRootPolicyOptions);
        default = { };
        description = ''
          Policies for temporary GC roots(e.g. result and direnv).
        '';
      };
      profile_policies = lib.mkOption {
        type = with lib.types; attrsOf (submodule profilePolicyOptions);
        default = { };
        description = ''
          Profile GC root policies.
        '';
      };
    };
  };
  commonPolicyOptions = {
    options = {
      enable = lib.mkEnableOption "this policy";
    };
  };
  temporaryRootPolicyOptions = {
    imports = [ commonPolicyOptions ];
    options = {
      path_regex = lib.mkOption {
        type = lib.types.str;
        description = ''
          Regex pattern to match the GC root path.
        '';
      };
      period = lib.mkOption {
        type = with lib.types; nullOr str;
        default = null;
        description = ''
          Retention period for the GC roots matched by this policy.
        '';
      };
      priority = lib.mkOption {
        type = lib.types.int;
        default = 100;
        description = ''
          Priority of this policy.

          Lower number means higher priority, if multiple policies monitor the
          same path, the one with higher priority will be applied.
        '';
      };
      filter = lib.mkOption {
        type = with lib.types; nullOr (submodule filterOptions);
        default = null;
        description = ''
          External filter program to further filter GC roots matched by this policy.
        '';
      };
      ignore_prefixes = lib.mkOption {
        type = with lib.types; nullOr (listOf str);
        default = null;
        description = ''
          List of path prefixes to ignore.

          If null is specified, angrr builtin settings will be used.
        '';
      };
      ignore_prefixes_in_home = lib.mkOption {
        type = with lib.types; nullOr (listOf str);
        default = null;
        description = ''
          Path prefixes to ignore under home directory.

          If null is specified, angrr builtin settings will be used.
        '';
      };
    };
  };
  profilePolicyOptions = {
    imports = [ commonPolicyOptions ];
    options = {
      profile_paths = lib.mkOption {
        type = with lib.types; listOf str;
        description = ''
          Paths to the Nix profile.

          When angrr runs in owned_only mode, and the option begins with `~`,
          it will be expanded to the home directory of the current user.

          When angrr does not run in owned_only mode, and the option begins with `~`,
          it will be expanded to the home of all users discovered respectively.
        '';
      };
      keep_since = lib.mkOption {
        type = with lib.types; nullOr str;
        default = null;
        description = ''
          Retention period for the GC roots in this profile.
        '';
      };
      keep_latest_n = lib.mkOption {
        type = with lib.types; nullOr int;
        default = null;
        description = ''
          Keep the latest N GC roots in this profile.
        '';
      };
      keep_current_system = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to keep the current system generation. Only useful for system profiles.
        '';
      };
      keep_booted_system = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to keep the last booted system generation. Only useful for system profiles.
        '';
      };
    };
  };
  filterOptions = {
    options = {
      program = lib.mkOption {
        type = lib.types.str;
        description = ''
          Path to the external filter program.
        '';
      };
      arguments = lib.mkOption {
        type = with lib.types; listOf str;
        default = [ ];
        description = ''
          Extra command-line arguments pass to the external filter program.
        '';
      };
    };
  };

  # toml.generate does not support null values, we need to filter them out first
  filteredConfig = lib.filterAttrsRecursive (name: value: value != null) cfg.config;
  originalConfigFile = toml.generate "angrr.toml" filteredConfig;
  validatedConfigFile = pkgs.runCommand "angrr-config.toml" { } ''
    ${lib.getExe cfg.package} validate --config "${originalConfigFile}" > $out
  '';
in
{
  meta.maintainers = pkgs.angrr.meta.maintainers;
  imports = [
    (lib.mkRemovedOptionModule [ "services" "angrr" "period" ] ''
      This option is removed since angrr 0.2.0.
      Please use `services.angrr.config` to configure retention policies through configuration file.

      `services.angrr.period` is replaced by the following two options in configuration file:
      * `services.angrr.config.temporary_root_policies.result.period`
      * `services.angrr.config.temporary_root_policies.direnv.period`
    '')
    (lib.mkRenamedOptionModule
      [ "services" "angrr" "removeRoot" ]
      [ "services" "angrr" "config" "remove_root" ]
    )
    (lib.mkRenamedOptionModule
      [ "services" "angrr" "ownedOnly" ]
      [ "services" "angrr" "config" "owned_only" ]
    )
  ];
  options = {
    services.angrr = {
      enable = lib.mkEnableOption "angrr";
      package = lib.mkPackageOption pkgs "angrr" { };
      logLevel = lib.mkOption {
        type =
          with lib.types;
          enum [
            "off"
            "error"
            "warn"
            "info"
            "debug"
            "trace"
          ];
        default = "info";
        description = ''
          Set the log level of angrr.
        '';
      };
      extraArgs = lib.mkOption {
        type = with lib.types; listOf str;
        default = [ ];
        description = ''
          Extra command-line arguments pass to angrr.
        '';
      };
      config = lib.mkOption {
        type = lib.types.submodule configOptions;
        description = ''
          Global configuration for angrr in TOML format.
        '';
      };
      configFile = lib.mkOption {
        type = with lib.types; nullOr path;
        default = validatedConfigFile;
        defaultText = "TOML file generated from `services.angrr.config`";
        description = ''
          Path to the angrr configuration file in TOML format.

          If not set, the configuration generated from `services.angrr.config` will be used.
          If specified, `services.angrr.config` will be ignored.
        '';
      };
      enableNixGcIntegration = lib.mkOption {
        type = lib.types.bool;
        description = ''
          Whether to enable nix-gc.service integration
        '';
      };
      timer = {
        enable = lib.mkEnableOption "angrr timer";
        dates = lib.mkOption {
          type = lib.types.str;
          default = "03:00";
          description = ''
            How often or when the retention policy is performed.
          '';
        };
      };
    };
    programs.direnv.angrr = {
      enable = lib.mkEnableOption "angrr direnv integration" // {
        default = true;
        example = false;
      };
      autoUse = lib.mkOption {
        type = lib.types.bool;
        default = true;
        example = false;
        description = ''
          Whether to automatically use angrr before loading .envrc.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      {
        assertions = [
          {
            assertion = cfg.enableNixGcIntegration -> config.nix.gc.automatic;
            message = "angrr nix-gc.service integration requires `nix.gc.automatic = true`";
          }
        ];
        services.angrr.enableNixGcIntegration = lib.mkDefault config.nix.gc.automatic;
      }

      {
        # Provide reasonable default policy configurations to merge with users' config
        # Users can easily disable them by setting `enable = false` in their config
        services.angrr.config = {
          temporary_root_policies = {
            result = {
              enable = lib.mkDefault true;
              path_regex = "/result[^/]*$";
            };
          };
          profile_policies = {
            system = {
              enable = lib.mkDefault false;
              profile_paths = [ "/nix/var/nix/profiles/system" ];
              keep_booted_system = true;
              keep_current_system = true;
            };
            user = {
              enable = lib.mkDefault false;
              profile_paths = [
                "~/.local/state/nix/profiles/profile"
                "/nix/var/nix/profiles/per-user/root/profile"
              ];
              keep_booted_system = false;
              keep_current_system = false;
            };
          };
        };

        environment.etc."angrr/config.toml".source = cfg.configFile;

        systemd.services.angrr = {
          description = "Auto Nix GC Roots Retention";
          script = ''
            ${lib.getExe cfg.package} run \
              --log-level "${cfg.logLevel}" \
              --no-prompt \
              ${lib.escapeShellArgs cfg.extraArgs}
          '';
          serviceConfig = {
            Type = "oneshot";
          };
        };

        environment.systemPackages = [ cfg.package ];
      }

      (lib.mkIf cfg.timer.enable {
        systemd.timers.angrr = {
          timerConfig = {
            OnCalendar = cfg.timer.dates;
          };
          wantedBy = [ "timers.target" ];
        };
      })

      (lib.mkIf cfg.enableNixGcIntegration {
        systemd.services.angrr = {
          wantedBy = [ "nix-gc.service" ];
          before = [ "nix-gc.service" ];
        };
      })

      (lib.mkIf (config.programs.direnv.enable && direnvCfg.enable) {
        services.angrr.config.temporary_root_policies.direnv = {
          enable = lib.mkDefault true;
          path_regex = "/\\.direnv/";
        };
        environment.etc."direnv/lib/angrr.sh".source = "${cfg.package}/share/direnv/lib/angrr.sh";
        programs.direnv.direnvrcExtra = lib.mkIf direnvCfg.autoUse ''
          use angrr
        '';
      })
    ]
  );
}
