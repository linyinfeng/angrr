{
  config,
  lib,
  ...
}:
let
  cfg = config.services.angrr;
  direnvCfg = config.programs.direnv.angrr;
in
{
  imports = [ ../shared/options.nix ];
  options = {
    services.angrr.timer = {
      enable = lib.mkEnableOption "angrr timer";
      dates = lib.mkOption {
        type = with lib.types; listOf (attrsOf int);
        default = [
          {
            Hour = 3;
            Minute = 0;
          }
        ];
        description = ''
          How often or when the retention policy is performed.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      {
        environment.etc."angrr/config.toml".source = cfg.configFile;

        launchd.daemons.angrr = {
          script = ''
            ${lib.getExe cfg.package} run \
              --log-level "${cfg.logLevel}" \
              --no-prompt \
              ${lib.escapeShellArgs cfg.extraArgs}
          '';
          serviceConfig.RunAtLoad = false;
        };

        environment.systemPackages = [ cfg.package ];
      }

      (lib.mkIf cfg.timer.enable {
        launchd.daemons.angrr.serviceConfig.StartCalendarInterval = cfg.timer.dates;
      })

      (lib.mkIf (config.programs.direnv.enable && direnvCfg.enable) {
        environment.etc."direnv/lib/angrr.sh".source = "${cfg.package}/share/direnv/lib/angrr.sh";
        programs.direnv.direnvrcExtra = lib.mkIf direnvCfg.autoUse ''
          _angrr_auto_use "$@"
        '';
      })

      # When period is set, configure a preset retention policy
      # Users can still override settings via services.angrr.settings
      (lib.mkIf (cfg.period != null) {
        services.angrr.settings = {
          temporary-root-policies = {
            direnv = {
              path-regex = "/\\.direnv/";
              period = cfg.period;
            };
            result = {
              path-regex = "/result[^/]*$";
              period = cfg.period;
            };
          };
          profile-policies = {
            system = {
              # nix-darwin also put system profile here
              # See https://github.com/nix-darwin/nix-darwin/blob/master/pkgs/nix-tools/default.nix
              # (https://github.com/nix-darwin/nix-darwin/blob/7b1d394e7d9112d4060e12ef3271b38a7c43e83b/pkgs/nix-tools/default.nix#L7)
              profile-paths = [ "/nix/var/nix/profiles/system" ];
              keep-since = cfg.period;
              keep-booted-system = true;
              keep-current-system = true;
            };
            user = {
              profile-paths = [
                "~/.local/state/nix/profiles/profile"
                "/nix/var/nix/profiles/per-user/root/profile"
              ];
              keep-since = cfg.period;
            };
          };
        };
      })
    ]
  );
}
