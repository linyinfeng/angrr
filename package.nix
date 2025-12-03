{
  lib,
  stdenv,
  rustPlatform,
  installShellFiles,
  nixosTests,
  testers,
  nix-update-script,
  clippy,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "angrr";

  # flake only stuff

  inherit (cargoToml.package) version;
  src =
    with lib.fileset;
    toSource {
      root = ./.;
      fileset = unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
        ./direnv
      ];
    };
  cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };

  nativeCheckInputs = [ clippy ];
  postCheck = ''
    cargo clippy --all-targets -- --deny warnings
  '';

  # contents below should be upstreamed to nixpkgs eventually

  nativeBuildInputs = [ installShellFiles ];
  postInstall = ''
    install -m400 -D ./direnv/angrr.sh $out/share/direnv/lib/angrr.sh
  ''
  + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd angrr \
      --bash <($out/bin/angrr completion bash) \
      --fish <($out/bin/angrr completion fish) \
      --zsh  <($out/bin/angrr completion zsh)
  '';

  passthru = {
    tests = {
      module = nixosTests.angrr;
      version = testers.testVersion {
        package = finalAttrs.finalPackage;
      };
    };
    updateScript = nix-update-script { };
  };

  meta = {
    description = "Tool for auto Nix GC roots retention";
    homepage = "https://github.com/linyinfeng/angrr";
    license = [ lib.licenses.mit ];
    maintainers = with lib.maintainers; [ yinfeng ];
    platforms = with lib.platforms; linux ++ darwin;
    mainProgram = "angrr";
  };
})
