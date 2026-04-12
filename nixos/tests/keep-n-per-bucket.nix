{ pkgs, ... }:
let
  drvForTest =
    name:
    pkgs.runCommand "angrr-test-${name}" { } ''
      mkdir --parents "$out"
      echo "${name}" >"$out/${name}"
    '';
in
{
  name = "angrr-keep-n-per-bucket";
  nodes = {
    machine = {
      services.angrr = {
        enable = true;
        logLevel = "debug";
        settings = {
          profile-policies = {
            system = {
              profile-paths = [ "/nix/var/nix/profiles/system" ];
              keep-n-per-bucket = [
                {
                  bucket-amount = 3;
                  bucket-window = "7 days";
                }
                {
                  bucket-amount = 2;
                  bucket-window = "30 days";
                }
              ];
            };
          };
        };
      };
      # `angrr.service` integrates to `nix-gc.service` by default
      nix.gc.automatic = true;

      # Create a normal nix user for test
      users.users.normal.isNormalUser = true;
      # For `nix build /run/current-system --out-link`,
      # `nix-build` does not support this use case.
      nix.settings.experimental-features = [ "nix-command" ];

      # Add some store paths to machine for test
      environment.etc."drvs-for-test".text = ''
        ${drvForTest "drv1"}
        ${drvForTest "drv2"}
        ${drvForTest "drv3"}
        ${drvForTest "drv4"}
        ${drvForTest "drv5"}
        ${drvForTest "drv6"}
        ${drvForTest "drv7"}
        ${drvForTest "drv8"}
        ${drvForTest "drv9"}
        ${drvForTest "drv10"}
        ${drvForTest "drv11"}
        ${drvForTest "drv12"}
        ${drvForTest "drv13"}
        ${drvForTest "drv14"}
      '';

      # Unit start limit workaround
      systemd.services.angrr.unitConfig.StartLimitBurst = 10;
    };
  };

  testScript = /* python */ ''
    start_all()

    machine.wait_for_unit("default.target")
    machine.systemctl("stop nix-gc.timer")

    # Create some GC roots
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv1"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv2"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv3"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv4"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv5"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv6"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv7"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv8"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv9"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv10"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv11"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv12"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv13"}")
    machine.succeed("date -s '5 days'")
    machine.succeed("nix-env --profile /nix/var/nix/profiles/system --set ${drvForTest "drv14"}")
    machine.succeed("date -s '5 days'")

    machine.systemctl("start angrr.service")
    machine.succeed("readlink /nix/var/nix/profiles/system-14-link") # keep (first of weekly)
    machine.succeed("readlink /nix/var/nix/profiles/system-13-link") # keep (second of weekly)
    machine.succeed("readlink /nix/var/nix/profiles/system-12-link") # keep (third of weekly)
    machine.succeed("readlink /nix/var/nix/profiles/system-11-link") # keep (first of monthly)
    machine.succeed("test ! -e /nix/var/nix/profiles/system-10-link")
    machine.succeed("readlink /nix/var/nix/profiles/system-9-link") # keep (second of monthly)
    machine.succeed("test ! -e /nix/var/nix/profiles/system-8-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-7-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-6-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-5-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-4-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-3-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-2-link")
    machine.succeed("test ! -e /nix/var/nix/profiles/system-1-link")
  '';
}
