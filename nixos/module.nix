{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.services.angrr;
in
{
  options = {
    services.angrr = {
      enable = lib.mkEnableOption "angrr";
      package = lib.mkPackageOption pkgs "angrr" { };
      dates = lib.mkOption {
        type = with lib.types; str;
        default = "03:00";
        description = ''
          How often or when the retention policy is performed.
        '';
      };
      period = lib.mkOption {
        type = with lib.types; str;
        default = "7d";
        example = "2weeks";
        description = ''
          The retention period of auto GC roots.
        '';
      };
      extraArgs = lib.mkOption {
        type = with lib.types; listOf str;
        default = [ ];
        description = ''
          Extra command-line arguments pass to angrr.
        '';
      };
    };
  };
  config = lib.mkIf cfg.enable {
    systemd.timers.angrr = {
      timerConfig = {
        OnCalendar = cfg.dates;
      };
      wantedBy = [ "timers.target" ];
    };
    systemd.services.angrr = {
      description = "Auto Nix GC Roots Retention";
      script = ''
        ${cfg.package}/bin/angrr run \
          --period "${cfg.period}" \
          --no-prompt ${lib.escapeShellArgs cfg.extraArgs}
      '';
      serviceConfig = {
        Type = "oneshot";
      };
    };
  };
}
