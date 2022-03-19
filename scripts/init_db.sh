#!/bin/sh
set -e

# Wait database to accept connections.
/bin/sh $SCRIPTS_DIR/wait_for_db.sh

echo "DATABASE_URL is set to: ${DATABASE_URL}"

# Create database defined at $DATABASE_URL.
echo "Creating database"
sqlx database create
echo "Finished creating database"

# Run sql migration scripts from $WORKSPACE/migrations.
echo "Start migration"
sqlx migrate run
echo "Finished migration"
