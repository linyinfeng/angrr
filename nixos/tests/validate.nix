{ ... }:
{
  name = "angrr-validate";
  nodes = {
    machine =
      { pkgs, ... }:
      {
        environment.systemPackages = [ pkgs.angrr ];
      };
  };

  testScript = /* python */ ''
    start_all()
    machine.wait_for_unit("default.target")

    with subtest("generate empty toml"):
      machine.succeed(":> empty.toml")

    with subtest("should not be affected with LOG_STYLE"):
      machine.succeed("ANGRR_LOG_STYLE=foo angrr validate -c empty.toml")

    with subtest("should be affected with other stuff"):
      machine.fail("ANGRR_MEOW_MEOW=foo angrr validate -c empty.toml")
  '';
}
