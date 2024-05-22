with (import ./pkgs.nix);
let
  ci-deps = pkgs.callPackage ./deps.nix { inherit pkgs; env = "ci"; };

in rec {
  base = pkgs.dockerTools.buildLayeredImage {
    name = "base";
    tag = "latest";

    extraCommands = ''
      mkdir -m 1777 tmp
      mkdir -p usr/bin run bin lib64
      ln -s ${pkgs.coreutils}/bin/env usr/bin/env
    '';

    contents = [
      pkgs.bash pkgs.coreutils pkgs.gnugrep
      pkgs.jq
      pkgs.curl
      pkgs.cacert
    ];

    config = {
      Env = [
        "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
        "LC_ALL=C.UTF-8" "LANG=C.UTF-8"
      ];
      Entrypoint = [ "${pkgs.bash}/bin/bash" "-c" ];
    };
  };

  ci-tools = pkgs.dockerTools.buildLayeredImage {
    name = "ci-tools";
    tag = "latest";
    fromImage = base;
    contents = ci-deps.packages;
  };
}
