{ ... }:
{
  name = "angrr-validate";
  nodes = {
    machine =
      { pkgs, ... }:
      {
        environment.systemPackages = [ pkgs.angrr ];
        environment.etc."angrr/config.toml".source = ../../etc/example-config.toml;
      };
  };

  testScript = ''
    import tomllib

    start_all()
    machine.wait_for_unit("default.target")

    with subtest("Global config"):
      machine.succeed("angrr validate")

    with subtest("Environment variable"):
      data = tomllib.loads(machine.succeed("ANGRR_store=/tmp/nix/store angrr validate"))
      assert data["store"] == "/tmp/nix/store"

    with subtest("Log style environment variable"):
      machine.succeed("ANGRR_LOG_STYLE=foo angrr validate")

    with subtest("Invalid environment variable"):
      machine.fail("ANGRR_MEOW_MEOW=foo angrr validate")

    with subtest("Empty configuration file"):
      machine.succeed("touch empty.toml")
      default = tomllib.loads(machine.succeed("angrr validate --no-global-config"))
      empty = tomllib.loads(machine.succeed("angrr validate --no-global-config --config empty.toml"))
      assert default == empty
  '';
}
