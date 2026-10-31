{
  description = "The Mortal Estate";

  inputs.burrow.url = "git+https://github.com/FieldmouseWorks/burrow";

  # `nix develop` for the tools, `nix flake check` for the gate (see burrow's
  # lib/project.nix): rustfmt, clippy and the workspace's tests on
  # rust-toolchain.toml's toolchain, and every local link and #fragment the
  # Markdown points at.
  outputs =
    { self, burrow }:
    burrow.lib.project {
      inherit self;
      # The client links udev (gamepads) and Wayland, which its build
      # scripts find with pkg-config.
      rust = {
        nativeBuildInputs = pkgs: [ pkgs.pkg-config ];
        buildInputs = pkgs: [
          pkgs.udev
          pkgs.wayland
        ];
      };
      packages = pkgs: [ pkgs.lychee ];
      checks = pkgs: {
        # lychee, offline, over every Markdown file. External links aren't
        # checked, but lychee won't start without CA certificates.
        links = pkgs.runCommand "links" { nativeBuildInputs = [ pkgs.lychee ]; } ''
          export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
          cd ${self}
          lychee --offline --no-progress --include-fragments \
            --root-dir "$PWD" \
            './**/*.md'
          touch $out
        '';
      };
    };
}
