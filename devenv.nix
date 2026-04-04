{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  packages =
    with pkgs;
    [ lld ] ++ [ inputs.nixpkgsUnstable.legacyPackages.${pkgs.stdenv.system}.opencode ];
  languages = {
    rust = {
      enable = true;
    };
  };
}
