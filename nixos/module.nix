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
      removeRoot = lib.mkOption {
        type = with lib.types; bool;
        default = false;
        description = ''
          Whether to pass the `--remove-root` option to angrr.
        '';
      };
      ownedOnly = lib.mkOption {
        type = with lib.types; bool;
        default = false;
        description = ''
          Control the `--remove-root=<true|false>` option of angrr.
        '';
        apply = b: if b then "true" else "false";
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
          ${lib.optionalString cfg.removeRoot "--remove-root"} \
          --owned-only="${cfg.ownedOnly}" \
          --no-prompt ${lib.escapeShellArgs cfg.extraArgs}
      '';
      serviceConfig = {
        Type = "oneshot";
      };
    };
  };
}
