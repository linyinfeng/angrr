{
  lib,
  pkgs,
  ...
}:
{
  options = {
    services.angrr = {
      enable = lib.mkEnableOption "angrr";
      package = lib.mkPackageOption pkgs "angrr" { };
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
      logLevel = lib.mkOption {
        type =
          with lib.types;
          enum [
            "off"
            "error"
            "warn"
            "info"
            "debug"
            "trace"
          ];
        default = "info";
        description = ''
          Set the log level of angrr.
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
    programs.direnv.angrr = {
      enable = lib.mkEnableOption "angrr direnv integration" // {
        default = true;
        example = false;
      };
      autoUse = lib.mkOption {
        type = lib.types.bool;
        default = true;
        example = false;
        description = ''
          Whether to automatically use angrr before loading .envrc.
        '';
      };
    };
  };
}
