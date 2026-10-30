{
  description = "The Mortal Estate";

  inputs.burrow.url = "git+https://github.com/FieldmouseWorks/burrow";

  # `nix develop` for the tools, `nix flake check` for the gate (see burrow's
  # lib/project.nix). The repo is a design doc until the skeleton lands, so the
  # gate is its own: every local link and #fragment the Markdown points at is
  # found. The skeleton adds `rust = { };` and the client's checks.
  outputs =
    { self, burrow }:
    burrow.lib.project {
      inherit self;
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
