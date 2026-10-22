{pkgs, ...}: {
  # channel = "stable" pulls the toolchain from rust-overlay
  # (declared in devenv.yaml), not nixpkgs.
  languages.rust = {
    enable = true;
    channel = "stable";
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };
}
