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
          database.passwordFile = pkgs.writeText "text" "password";
          secrets.hmac_secret = pkgs.writeText "text" "shared";
        };

        services.postgresql = {
          enable = true;
          initialScript = pkgs.writeText "pg-init-script.sql" ''
            CREATE ROLE waterwheel LOGIN PASSWORD 'password';
            CREATE DATABASE waterwheel OWNER waterwheel;
          '';
        };
      };

      testScript = ''
        start_all()
        machine.wait_for_unit("network-online.target")
        machine.wait_for_unit("waterwheel.service")
        machine.wait_for_open_port(8080)
        # create project
        machine.wait_until_succeeds("curl -X POST -H 'Content-Type: application/json' -d @${../sample/project.json} localhost:8080/api/projects -i")
        # create jobs
        machine.wait_until_succeeds("curl -X POST -H 'Content-Type: application/json' -d @${../sample/jobs/simple.json} localhost:8080/api/jobs -i")
      '';
    }
    {
      inherit pkgs;
      inherit (pkgs) system;
    };
}
