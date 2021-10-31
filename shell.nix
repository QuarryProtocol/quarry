{ pkgs }:
pkgs.mkShell {
  nativeBuiltInputs = (pkgs.lib.optionals pkgs.stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.AppKit
    pkgs.darwin.apple_sdk.frameworks.IOKit
    pkgs.darwin.apple_sdk.frameworks.Foundation
  ]);
  buildInputs = with pkgs; [
    (import ./ci.nix { inherit pkgs; })
    rustup
    cargo-deps
    gh
    spl-token-cli
  ];
}
