# UBL Gate service ops

## Local
```bash
make gate
curl -fsS http://127.0.0.1:4000/healthz
```

## Docker
```bash
docker build -f ops/gate/Dockerfile -t ubl-gate:local .
docker run --rm -p 4000:4000 --env UBL_GATE_BIND=0.0.0.0:4000 ubl-gate:local
curl -fsS http://127.0.0.1:4000/healthz
```

## Compose
```bash
docker compose -f ops/gate/docker-compose.yml up --build
curl -fsS http://127.0.0.1:4000/healthz
```

## Systemd
```bash
sudo install -D -m 0644 ops/gate/ubl-gate.service /etc/systemd/system/ubl-gate.service
sudo install -D -m 0644 ops/gate/env.example /etc/ubl-gate/ubl-gate.env
sudo systemctl daemon-reload
sudo systemctl enable --now ubl-gate
curl -fsS http://127.0.0.1:4000/healthz
```
