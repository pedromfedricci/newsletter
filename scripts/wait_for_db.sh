#!/bin/sh
set -e

echo "Postgres environment variables:"
printenv | grep ^PG

until pg_isready; do
    >&2 echo "Postgres is not availiable for connections yet"
    sleep 1
done

>&2 echo "Postgres is up and ready for connections"
