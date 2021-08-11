#!/usr/bin/env bash
echo "DATABASE_URL=${DATABASE_URL}"
sqlx database create
sqlx migrate run
