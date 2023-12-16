# Anything non-container related that is needed to develop and not wanted system-wide

{ pkgs ? import <nixpkgs> {}}:

pkgs.mkShell {
  packages = [ 
    pkgs.hello
  ];
}