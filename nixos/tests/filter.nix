{ pkgs, lib, ... }:
{
  name = "angrr-filter";
  nodes = {
    machine =
      { pkgs, ... }:
      {
        services.angrr = {
          enable = true;
          config = {
            temporary_root_policies = {
              direnv = {
                period = "0s";
              };
              result = {
                period = "0s";
                ignore_prefixes = [ "/tmp/ignore-directory" ];
                ignore_prefixes_in_home = [ "ignore-directory" ];
                filter = {
                  program = "jq";
                  arguments = [
                    "--exit-status"
                    ".path | test(\"/result-special-filter$\") | not"
                  ];
                };
              };
            };
          };
        };
        programs.direnv.enable = true;
        # Create two normal nix users for test
        users.users.user1.isNormalUser = true;
        users.users.user2.isNormalUser = true;
        # For `nix build /run/current-system --out-link`,
        # `nix-build` does not support this use case.
        nix.settings.experimental-features = [ "nix-command" ];
      };
  };

  testScript =
    let
      # Use /run/current-system so that we do not need to build anything new
      angrrCommand = pkgs.writeShellApplication {
        name = "angrr-test-run";
        runtimeInputs = [ pkgs.jq ]; # use angrr from path (from node pkgs)
        # use default --path-regex
        text = ''
          angrr run \
            --interactive=never \
            --output=/tmp/removed
        '';
      };
      user1GcRoots = [
        "/tmp/regex-not-match"
        "/tmp/.direnv/regex-match"
        "/tmp/_direnv/test"
        "/tmp/result"
        "/tmp/result-lib"
        "/tmp/ignore-directory/result"
        "/home/user1/ignore-directory/result"
        "/home/user1/result"
        "/home/user1/result-special-filter"
      ];
      user2GcRoots = [
        "/tmp/other-users"
        "/tmp/result2"
      ];
      rootGcRoots = [
        "/tmp/root-users"
        "/tmp/result3"
      ];
      expectedRemovedPaths = [
        "/tmp/.direnv/regex-match"
        "/tmp/result"
        "/tmp/result-lib"
        "/home/user1/result"
      ];
      expectedRemovedPathsFile = pkgs.writeText "expected-removed-paths" (
        lib.concatStringsSep "\n" expectedRemovedPaths
      );
      mkGcRoot =
        paths:
        pkgs.writeShellApplication {
          name = "make-gc-root";
          text = ''
            for path in ${lib.concatStringsSep " " paths}; do
              mkdir --parents "$(dirname "$path")" --verbose
              echo "linking $path..."
              nix build /run/current-system --out-link "$path"
            done
          '';
        };
      testScript = pkgs.writeShellApplication {
        name = "angrr-filter-test";
        text = ''
          su user1 --command "${lib.getExe (mkGcRoot user1GcRoots)}"
          su user2 --command "${lib.getExe (mkGcRoot user2GcRoots)}"
          "${lib.getExe (mkGcRoot rootGcRoots)}"
          su user1 --command "${lib.getExe angrrCommand}"
          echo "comparing removed paths..."
          diff --unified <(sort "${expectedRemovedPathsFile}") <(sort /tmp/removed)
          echo "done"
        '';
      };
    in
    ''
      start_all()
      machine.succeed("${lib.getExe testScript}")
    '';
}
