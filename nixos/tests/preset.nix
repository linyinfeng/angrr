{ ... }:
{
  name = "angrr-preset";
  nodes = {
    machine =
      { ... }:
      {
        services.angrr = {
          enable = true;
          period = "7d";
        };
        nix.gc.automatic = true;
        programs.direnv.enable = true;
        # For `nix build /run/current-system --out-link`,
        # `nix-build` does not support this use case.
        nix.settings.experimental-features = [ "nix-command" ];
      };
  };

  testScript = ''
    start_all()

    machine.wait_for_unit("default.target")

    machine.systemctl("stop nix-gc.timer")

    # Creates some auto gc roots
    # Use /run/current-system so that we do not need to build anything new
    machine.succeed("nix build /run/current-system --out-link /tmp/result-root-auto-gc-root-1")
    machine.succeed("nix build /run/current-system --out-link /tmp/result-root-auto-gc-root-2")

    machine.systemctl("start nix-gc.service")
    # No auto gc root will be removed
    machine.succeed("readlink /tmp/result-root-auto-gc-root-1")
    machine.succeed("readlink /tmp/result-root-auto-gc-root-2")

    # Change time to 8 days after (greater than 7d)
    machine.succeed("date -s '8 days'")

    # Touch GC roots `-2`
    machine.succeed("touch /tmp/result-root-auto-gc-root-2 --no-dereference")

    machine.systemctl("start nix-gc.service")
    # Only GC root `-1` is removed
    machine.succeed("test ! -e /tmp/result-root-auto-gc-root-1")
    machine.succeed("readlink  /tmp/result-root-auto-gc-root-2")
  '';
}
