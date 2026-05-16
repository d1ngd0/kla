# List what we can do
default:
    @just --list dev

# spins up a service with docker to test oauth against
oauth_dev:
    docker compose -f resources/oauth_service.yaml up -d 

# cleans up the docker service for local development against oauth
oauth_clean:
    docker compose -f resources/oauth_service.yaml down
