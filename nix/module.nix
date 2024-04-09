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
  port = lib.toInt (lib.last (lib.splitString ":" cfg.host));
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

    host = mkOption {
      default = "http://127.0.0.1:8080";
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

    secrets = with lib; {
      hmac_secret = mkOption {
        type = types.path;
        example = "/run/keys/hmac_secret";
        description = '''';
      };
      no_authz = mkOption {
        type = types.bool;
        default = true;
        description = '''';
      };
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
            pgsql = config.services.postgresql.port;
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
          if cfg.database.type == "pgsql"
          then "/run/postgresql"
          else null;
        defaultText = literalExpression "/run/postgresql";
        description = "Path to the unix socket file to use for authentication.";
      };
    };

    worker = lib.mkOption {
      description = "worker settings";
      default = {};
      type = lib.types.submodule {
        options = {
          enable = mkEnableOption "enable waterwheel worker";
        };
      };
    };
  };

  config = let
    commonService = {
      wantedBy = ["multi-user.target"];
      after = ["postgresql.service" "rabbitmq.service"];
      serviceConfig = {
        User = "${name}";
        Group = "${name}";
        ExecStop = "${pkgs.coreutils}/bin/kill -INT $MAINPID";
        LoadCredential = [
          "DATABASE_PASSWORD:${cfg.database.passwordFile}"
          "HMAC_SECRET:${cfg.secrets.hmac_secret}"
        ];
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
          SERVER_ADDR = cfg.host;
          NO_AUTHZ = toString cfg.secrets.no_authz;
        }
      );
    };
  in
    lib.mkMerge [
      {
        users.users.${name} = {
          isSystemUser = true;
          group = "${name}";
        };
        users.groups.${name} = {};
      }
      (lib.mkIf cfg.worker.enable {
        environment.systemPackages = [cfg.package];
        systemd.services.waterwheel-worker = lib.recursiveUpdate commonService {
          script = ''
            export WATERWHEEL_HMAC_SECRET=$(cat $CREDENTIALS_DIRECTORY/HMAC_SECRET)
            export WATERWHEEL_DB_URL=postgres://${cfg.database.user}:$(cat $CREDENTIALS_DIRECTORY/DATABASE_PASSWORD)@${cfg.database.host}:${toString config.services.postgresql.port}/${cfg.database.name}
            ${cfg.package}/bin/waterwheel worker > var/lib/${name}/worker.log;
          '';
          serviceConfig = {
            Type = "notify";
          };
          after = [
            "network-online.target"
            "waterwheel.service"
          ];
        };
      })
      (lib.mkIf cfg.enable {
        environment.systemPackages = [cfg.package];
        systemd.services.waterwheel =
          lib.recursiveUpdate commonService
          {
            script = ''
              export WATERWHEEL_HMAC_SECRET=$(cat $CREDENTIALS_DIRECTORY/HMAC_SECRET)
              export WATERWHEEL_DB_URL=postgres://${cfg.database.user}:$(cat $CREDENTIALS_DIRECTORY/DATABASE_PASSWORD)@${cfg.database.host}:${toString config.services.postgresql.port}/${cfg.database.name}
              ${cfg.package}/bin/waterwheel scheduler
            '';
            serviceConfig = {Type = "simple";};
          };

        networking.firewall = mkIf cfg.openFirewall {
          allowedTCPPorts = [port];
          allowedUDPPorts = [port];
        };

        services.rabbitmq = {
          enable = true;
        };

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
      })
    ];
}
