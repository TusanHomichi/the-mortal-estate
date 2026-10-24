{
  description = "The Mortal Estate";

  inputs.burrow.url = "git+https://github.com/FieldmouseWorks/burrow";

  # `nix develop` for the toolchains, `nix flake check` for the gate (see
  # burrow's lib/project.nix). tools/run_verification.py stays the complete
  # verification authority; this gate is the hermetic part of it.
  outputs = { self, burrow }: burrow.lib.project {
    inherit self;
    rust = { };
    python = { dirs = [ "deploy" "tests" "tools" ]; };
    node = { };
  };
}
