{
  lib,
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

  inherit (cargoToml.workspace.package) version;
  src =
    with lib.fileset;
    toSource {
      root = ./.;
      fileset = unions [
        ./angrr
        ./xtask
        ./Cargo.toml
        ./Cargo.lock
        ./.cargo
        ./etc

        ./direnv
      ];
    };
  cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };

  nativeCheckInputs = [ clippy ];
  postCheck = ''
    cargo clippy --all-targets -- --deny warnings
  '';

  # contents below should be upstreamed to nixpkgs eventually

  buildAndTestSubdir = "angrr";

  nativeBuildInputs = [ installShellFiles ];
  postBuild = ''
    mkdir --parents build/{man-pages,shell-completions}
    cargo xtask man-pages --out build/man-pages
    cargo xtask shell-completions --out build/shell-completions
  '';
  postInstall = ''
    install -m400 -D ./direnv/angrr.sh $out/share/direnv/lib/angrr.sh
    installManPage build/man-pages/*
    installShellCompletion --cmd angrr \
      --bash build/shell-completions/angrr.bash \
      --fish build/shell-completions/angrr.fish \
      --zsh  build/shell-completions/_angrr
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
    description = "Temporary GC Roots Cleaner";
    homepage = "https://github.com/linyinfeng/angrr";
    license = [ lib.licenses.mit ];
    maintainers = with lib.maintainers; [ yinfeng ];
    platforms = with lib.platforms; linux ++ darwin;
    mainProgram = "angrr";
  };
})
