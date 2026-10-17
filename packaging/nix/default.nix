{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  makeWrapper,
  libxcb,
  libxkbcommon,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
  wayland,
  libGL,
  vulkan-loader,
}:

let
  workspace = (lib.importTOML ../../Cargo.toml).workspace.package;

  # Loaded with dlopen at run time rather than linked, so a plain `cargo build`
  # binary needs them on LD_LIBRARY_PATH; the wrapper below provides that.
  runtimeLibs = [
    libGL
    libxkbcommon
    wayland
    libx11
    libxcursor
    libxi
    libxrandr
    libxcb
    vulkan-loader
  ];
in
rustPlatform.buildRustPackage {
  pname = "tsumiba";
  inherit (workspace) version;

  src = lib.cleanSourceWith {
    src = ../..;
    filter =
      path: type:
      let
        base = baseNameOf path;
      in
      !(builtins.elem base [
        "target"
        "result"
        ".git"
        "docs-site"
        "extension"
        "node_modules"
      ]);
  };

  cargoLock.lockFile = ../../Cargo.lock;
  cargoBuildFlags = [
    "-p"
    "tsumiba"
  ];

  # The test suite starts real processes and sockets; it runs in CI instead.
  doCheck = false;

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];
  buildInputs = [
    libxcb
    libxkbcommon
    libx11
    wayland
  ];

  postInstall = ''
    install -Dm644 packaging/linux/tsumiba.desktop $out/share/applications/tsumiba.desktop
    for size in 16 32 48 64 128 256 512; do
      install -Dm644 assets/icons/tsumiba-$size.png \
        $out/share/icons/hicolor/''${size}x''${size}/apps/tsumiba.png
    done
    install -Dm644 assets/tsumiba.svg $out/share/icons/hicolor/scalable/apps/tsumiba.svg
  '';

  postFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    wrapProgram $out/bin/tsumiba \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
  '';

  meta = {
    description = "A cross-platform memo and file shelf that slides in from the screen edge";
    homepage = "https://hjosugi.github.io/tsumiba/";
    license = lib.licenses.mit;
    mainProgram = "tsumiba";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
