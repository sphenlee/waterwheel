{
  config,
  lib,
  pkgs,
  options,
  ...
}: let
  name = "waterwheel-worker";
  cfg = config.services.waterwheel-worker;
in {
  options.services.waterwheel-worker =
    options.services.waterwheel
    // {
      server = lib.mkOption {
        description = "server settings";
        default = {};
        type = lib.types.submodule {
          options = {
            host = lib.mkOption {
              type = lib.types.str;
              default = "http://localhost:8080";
              description = "which waterwheel server address to use";
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

              passwordFile = mkOption {
                type = types.nullOr types.path;
                default = config.services.waterwheel.database.passwordFile;
              };
            };

            hmac_secret = lib.mkOption {
              type = lib.types.path;
              default = config.services.waterwheel.secrets.hmac_secret;
              description = "which waterwheel server address to use";
            };
            no_authz = lib.mkOption {
              type = lib.types.bool;
              default = config.services.waterwheel.secrets.no_authz;
            };
          };
        };
      };
    };
  config = let
    commonService = {
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        Type = "notify";
        Restart = "always";
        RestartSec = "10";
        DynamicUser = true;
        ExecStop = "${pkgs.coreutils}/bin/kill -INT $MAINPID";
        NoNewPrivileges = true;
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        RestrictNamespaces = true;
        LogsDirectory = name;
        RuntimeDirectory = name;
        StateDirectory = name;
        LoadCredential = [
          "DATABASE_PASSWORD:${config.services.waterwheel.database.passwordFile}"
          "HMAC_SECRET:${config.services.waterwheel.secrets.hmac_secret}"
        ];
      };
    };
  in
    lib.mkMerge [
      (lib.mkIf cfg.enable {
        environment.systemPackages = [cfg.package];
        systemd.services.waterwheel-worker =
          commonService
          // {
            script = ''
              export WATERWHEEL_NO_AUTHZ=${toString cfg.server.no_authz}
              export WATERWHEEL_SERVER_ADDR=${cfg.server.host}
              export WATERWHEEL_HMAC_SECRET=$(cat $CREDENTIALS_DIRECTORY/HMAC_SECRET)
              export WATERWHEEL_DB_URL=postgres://${cfg.server.database.user}:$(cat $CREDENTIALS_DIRECTORY/DATABASE_PASSWORD)@${cfg.database.host}:${toString config.services.postgresql.port}/${cfg.server.database.name}
              ${cfg.package}/bin/waterwheel worker > var/lib/${name}/worker.log;
            '';
            after = [
              "network-online.target"
              "waterwheel.service"
            ];
          };
      })
    ];
}
