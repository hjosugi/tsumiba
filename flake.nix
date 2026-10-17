{
  description = "tsumiba, a memo and file shelf that slides in from the screen edge";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: import nixpkgs { inherit system; };

      # Libraries the window system and the GL/Vulkan loaders are dlopen'd from
      # at run time. They are not link-time dependencies, so they have to be on
      # LD_LIBRARY_PATH for a binary built outside of Nix.
      runtimeLibs =
        pkgs:
        pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux (
          with pkgs;
          [
            libGL
            libxkbcommon
            vulkan-loader
            wayland
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
          ]
        );
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        rec {
          tsumiba = pkgs.callPackage ./packaging/nix/default.nix { };
          default = tsumiba;
        }
      );

      apps = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = {
            type = "app";
            program = pkgs.lib.getExe self.packages.${system}.default;
            meta.description = "Run the tsumiba memo and file shelf";
          };
        }
      );

      checks = forAllSystems (system: {
        package = self.packages.${system}.default;
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                cargo
                clippy
                rust-analyzer
                rustc
                rustfmt
                pkg-config
                mdbook
                nodejs
                zip
                jq
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux (
                with pkgs;
                [
                  libx11
                  libxcb
                  libxkbcommon
                  # Headless GUI checks: a virtual display, input injection and
                  # a screenshot tool.
                  xvfb-run
                  xdotool
                  imagemagick
                  mesa
                ]
              );
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
          };
        }
      );
    };
}
