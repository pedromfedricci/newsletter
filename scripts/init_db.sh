#!/usr/bin/env bash
echo "DATABASE_RUL=${DATABASE_URL}"
sqlx database create
sqlx migrate run
