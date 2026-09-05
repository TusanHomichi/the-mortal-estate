"""Dedicated user units and a private nginx instance; no host configuration edits."""
from common import UNITS, write


def unit(description, command, *, after="", requires="", memory="1G", cpu="100%", extra=""):
    return f"""[Unit]
Description={description}
After={after}
Requires={requires}
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=exec
ExecStart={command}
Restart=on-failure
RestartSec=3
TimeoutStopSec=20
UMask=0077
NoNewPrivileges=yes
MemoryMax={memory}
CPUQuota={cpu}
TasksMax=128
{extra}
[Install]
WantedBy=default.target
"""


def install_units(site):
    root, ports = site.root, site.ports
    for name in UNITS:
        destination = site.units / (name + ".service")
        if destination.exists() and str(root) not in destination.read_text():
            raise RuntimeError(f"service {name} already belongs to another installation")
    write(site.units / (UNITS[0] + ".service"), unit("TME private development PostgreSQL",
          f"{site.pg_bin}/postgres -D {site.data}", extra="KillSignal=SIGINT\n"), 0o644)
    credentials = site.config / "credentials"
    write(site.units / (UNITS[1] + ".service"), unit("TME private development authority",
          f"{site.current}/bin/tme-server serve", after=UNITS[0] + ".service", requires=UNITS[0] + ".service",
          extra=f"WorkingDirectory={site.current}\nEnvironmentFile={site.config}/server.env\n"
                f"LoadCredential=database-url:{credentials}/database-url\n"
                f"LoadCredential=auth-database-url:{credentials}/auth-database-url\n"), 0o644)
    write(site.units / (UNITS[2] + ".service"), unit("TME private development HTTPS",
          f"/usr/sbin/nginx -p {root}/ -c {site.config}/nginx.conf -g 'daemon off;'",
          after=UNITS[1] + ".service", memory="256M", cpu="50%"), 0o644)
    write(site.config / "nginx.conf", f"""worker_processes 1;
pid {root}/nginx.pid;
error_log stderr warn;
events {{ worker_connections 128; }}
http {{
    include /etc/nginx/mime.types;
    default_type application/octet-stream;
    access_log off;
    client_body_temp_path {root}/nginx-temp/client;
    proxy_temp_path {root}/nginx-temp/proxy;
    server {{
        listen 127.0.0.1:{ports['https']} ssl;
        server_name localhost;
        ssl_certificate {site.config}/tls/current/server.pem;
        ssl_certificate_key {site.config}/tls/current/server.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        add_header X-Content-Type-Options nosniff always;
        add_header Cache-Control no-store always;
        client_max_body_size 64k;
        root {site.current}/web;
        location /internal/ {{ return 404; }}
        location /health/ {{ proxy_pass http://127.0.0.1:{ports['server']}; }}
        location /v3/ {{
            proxy_pass http://127.0.0.1:{ports['server']};
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_read_timeout 120s;
            proxy_buffering off;
        }}
        location / {{ try_files $uri $uri/ =404; }}
    }}
}}
""")
    for child in ("client", "proxy"):
        (root / "nginx-temp" / child).mkdir(parents=True, exist_ok=True)
