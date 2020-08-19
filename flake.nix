{
  description = "A collection of brainfuck implementaions";

  inputs.import-cargo.url = "github:edolstra/import-cargo";

  outputs = { self, nixpkgs, import-cargo }: {

    packages.x86_64-linux.brainfucks = let importCargo = import-cargo.builders.importCargo; in
      with import nixpkgs { system = "x86_64-linux"; };
      stdenv.mkDerivation {
        name = "brainfucks";
        src = self;
        nativeBuildInputs = [
          (importCargo { lockFile = ./Cargo.lock; inherit pkgs; }).cargoHome

          rustc cargo
        ];
        buildPhase = ''
          cargo build --release --offline
        '';
        installPhase = ''
          for bin in interpreter comp2c craneliftcomp; do
            install -Dm775 ./target/release/$bin $out/bin/$bin
          done
        '';
      };


    defaultPackage.x86_64-linux = self.packages.x86_64-linux.brainfucks;

  };
}
