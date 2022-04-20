{
  lib,
  pkgs,
  config,
  waterwheel,
  ...
}: let
  name = "waterwheel";
  inherit (lib) mkIf mkOption mkEnableOption;
  cfg = config.services.waterwheel;
in {
  options.services.waterwheel = {
    enable = mkEnableOption "enable waterwheel";

    package = mkOption {
      default = waterwheel;
      type = lib.types.package;
      description = ''
        Which waterwheel package to use.
      '';
    };

    address = mkOption {
      default = "http://localhost:8080";
      type = lib.types.str;
      description = ''
        Which waterwheel address to use.
      '';
    };

    openFirewall = mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open the listening port of the waterwheel port.";
    };

    database = with lib; {
      type = mkOption {
        type = types.enum ["pgsql"];
        default = "pgsql";
        description = "Database engine to use.";
      };

      host = mkOption {
        type = types.str;
        default = "localhost";
        description = "Database host address.";
      };

      port = mkOption {
        type = types.int;
        description = "Database host port.";
        default =
          {
            # mysql = 3306;
            pgsql = 5432;
          }
          .${cfg.database.type};
        defaultText = literalExpression "5432";
      };

      name = mkOption {
        type = types.str;
        default = "waterwheel";
        description = "Database name.";
      };

      user = mkOption {
        type = types.str;
        default = "waterwheel";
        description = "Database user.";
      };

      password = mkOption {
        type = types.str;
        default = "waterwheel";
        description = ''
          The password corresponding to <option>database.user</option>.
          Warning: this is stored in cleartext in the Nix store!
          Use <option>database.passwordFile</option> instead.
        '';
      };

      passwordFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        example = "/run/keys/waterwheel-dbpassword";
        description = ''
          A file containing the password corresponding to
          <option>database.user</option>.
        '';
      };

      socket = mkOption {
        type = types.nullOr types.path;
        default =
          if cfg.database.type == "mysql"
          then "/run/postgresql"
          else null;
        defaultText = literalExpression "/run/postgresql";
        description = "Path to the unix socket file to use for authentication.";
      };
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    systemd.services.waterwheel = {
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/waterwhell scheduler";
        ExecStop = "${pkgs.coreutils}/bin/kill -INT $MAINPID";
        DynamicUser = true;
        NoNewPrivileges = true;
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        RestrictNamespaces = true;
        LogsDirectory = name;
        RuntimeDirectory = name;
        StateDirectory = name;
      };
      environment = (
        lib.mapAttrs' (n: v: lib.nameValuePair "WATERWHEEL_${n}" (toString v))
        {
          DB_URL = "postgres://${cfg.database.user}:${cfg.password}@127.0.0.1/${cfg.database.name}";
          SERVER_ADDR = cfg.address;
          HMAC_SECRET = "shared secre";
          NO_AUTHZ = true;
        }
      );
    };

    networking.firewall = mkIf cfg.openFirewall {};

    services.postgresql = {
      enable = true;
      ensureDatabases = [cfg.database.name];
      ensureUsers = [
        {
          name = cfg.database.user;
          ensurePermissions = {"DATABASE ${cfg.database.name}" = "ALL PRIVILEGES";};
        }
      ];
    };
  };
}
