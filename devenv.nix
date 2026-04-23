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
    javascript = {
      enable = true;
      bun.enable = true;
    };
    rust = {
      enable = true;
    };
  };
}
