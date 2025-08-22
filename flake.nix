{
    description = "A basic flake providing a shell with rustup";
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay.url = "github:oxalica/rust-overlay";
    };

    outputs = {self, nixpkgs, flake-utils, rust-overlay}: 
        flake-utils.lib.eachDefaultSystem (system: 
            let 
                overlays = [ (import rust-overlay) ];
                pkgs = import nixpkgs {
                    inherit system overlays;
                };    
                in
                {
                    devShells.default = pkgs.mkShell {
                        buildInputs = with pkgs; [
                            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
                            curl
                            openssl
                            pkg-config
                            gcc
                            mold
                        ];
                shellHook = ''
                        ./setup_git_hook.sh
                    '';
                    };
                }
        );
}
