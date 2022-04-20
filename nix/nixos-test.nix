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
