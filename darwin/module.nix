{
  config,
  lib,
  ...
}:
let
  cfg = config.services.angrr;
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
        launchd.daemons.angrr = {
          script = ''
            ${cfg.package}/bin/angrr run \
              --log-level "${cfg.logLevel}" \
              --period "${cfg.period}" \
              ${lib.optionalString cfg.removeRoot "--remove-root"} \
              --owned-only="${cfg.ownedOnly}" \
              --no-prompt ${lib.escapeShellArgs cfg.extraArgs}
          '';
          serviceConfig.RunAtLoad = false;
        };
      }

      (lib.mkIf cfg.timer.enable {
        launchd.daemons.nix-gc.serviceConfig.StartCalendarInterval = cfg.timer.dates;
      })
    ]
  );
}
