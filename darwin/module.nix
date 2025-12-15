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
    ]
  );
}
