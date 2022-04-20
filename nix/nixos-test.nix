{
  makeTest,
  inputs,
  pkgs,
}: {
  waterwheel-vm-systemd =
    makeTest
    {
      name = "waterwheel-systemd";
      machine = {
        config,
        pkgs,
        lib,
        ...
      }: {
        environment.systemPackages = with pkgs; [];

        imports = [
          inputs.self.nixosModules.waterwheel
        ];

        virtualisation = {
          memorySize = 6000;
          cores = 4;
        };

        services.waterwheel = {
          enable = true;
          database.passwordFile = ./password;
          secrets.hmac_secret = ./hmac_secret;
        };

        services.postgresql = let
          password = lib.fileContents ./password;
        in {
          enable = true;
          initialScript = pkgs.writeText "pg-init-script.sql" ''
            CREATE ROLE waterwheel LOGIN PASSWORD '${password}';
            CREATE DATABASE waterwheel OWNER waterwheel;
          '';
        };
      };

      testScript = ''
        start_all()
        machine.wait_for_unit("network-online.target")
        machine.wait_for_unit("waterwheel.service")
        machine.wait_for_open_port(8080)
      '';
    }
    {
      inherit pkgs;
      inherit (pkgs) system;
    };
}
