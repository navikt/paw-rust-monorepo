# Docker for lokal utvikling

## Start containere
```bash
docker compose -f ./docker/postgres/docker-compose.yaml up -d
```
```bash
docker compose -f ./docker/kafka/docker-compose.yaml up -d
```
```bash
docker compose -f ./docker/mocks/docker-compose.yaml up -d
```

## Stopp containere
```bash
docker compose -f ./docker/postgres/docker-compose.yaml stop
```
```bash
docker compose -f ./docker/kafka/docker-compose.yaml stop
```
```bash
docker compose -f ./docker/mocks/docker-compose.yaml stop
```

## Slett containere
```bash
docker compose -f ./docker/postgres/docker-compose.yaml down -v
```
```bash
docker compose -f ./docker/kafka/docker-compose.yaml down -v
```
```bash
docker compose -f ./docker/mocks/docker-compose.yaml down -v
```
