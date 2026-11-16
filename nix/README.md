# Nix

Run commands from the checkout. See [Build and run](../README.md#build-and-run) for the command list.

- `assets.nix` installs the pinned Iosevka fonts and Carbon icons.
- `package.nix` vendors Cargo dependencies with crane, builds the release binary, and makes the macOS bundle.
- `checks.nix` runs isolated package, formatting, clippy, and workspace-test checks.
- `commands.nix` provides checkout commands, the development shell, real-server tests, and the cumulative gate.

`flake.nix` wires those modules to the locked inputs and the toolchain in `rust-toolchain.toml`.
The release binary retains its Nix asset path. `TAU_ASSETS` can override it at runtime.
Real-server tests stay outside `nix flake check` because they need HTTP and a local admin socket.
