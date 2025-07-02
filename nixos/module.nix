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
    services.angrr = {
      enableNixGcIntegration = lib.mkOption {
        type = with lib.types; bool;
        description = ''
          Whether to enable nix-gc.service integration
        '';
      };
      timer = {
        enable = lib.mkEnableOption "angrr timer";
        dates = lib.mkOption {
          type = with lib.types; str;
          default = "03:00";
          description = ''
            How often or when the retention policy is performed.
          '';
        };
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
        systemd.services.angrr = {
          description = "Auto Nix GC Roots Retention";
          script = ''
            ${cfg.package}/bin/angrr run \
              --log-level "${cfg.logLevel}" \
              --period "${cfg.period}" \
              ${lib.optionalString cfg.removeRoot "--remove-root"} \
              --owned-only="${cfg.ownedOnly}" \
              --no-prompt ${lib.escapeShellArgs cfg.extraArgs}
          '';
          serviceConfig = {
            Type = "oneshot";
          };
        };
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
    ]
  );
}
