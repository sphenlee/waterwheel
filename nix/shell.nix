{pkgs ? import <nixpkgs> {}}:
with pkgs; {
  default = mkShell {
    buildInputs = [
      (pkgs.rust-bin.selectLatestNightlyWith (toolchain:
        toolchain.default.override {
          extensions = ["rust-src"];
        }))
      nodejs
      just
      openssl
      pkg-config
    ];
    WATERWHEEL_DB_URL = "postgres://waterwheel:password@127.0.0.1:5432/waterwheel";
    WATERWHEEL_SERVER_ADDR = "http://localhost:8080";
    WATERWHEEL_HMAC_SECRET = "shared";
    WATERWHEEL_NO_AUTHZ = true;
  };
}
