#!/bin/sh
set -e

BAN_URL="https://adresse.data.gouv.fr/data/ban/adresses/latest/csv/adresses-france.csv.gz"
BAN_ADDRESS_COMPRESSED="/map/adresses-france.csv.gz"
BAN_ADDRESS="/map/adresses-france.csv"

apk update && apk add curl gzip

if [ -f "$BAN_ADDRESS" ]; then
    echo "✅ French BAN addresses already exist. Get started quickly."
else
    if [ ! -f "$BAN_ADDRESS_COMPRESSED" ]; then
        echo "📥 Downloading $BAN_URL..."
        curl -L "$BAN_URL" -o "$BAN_ADDRESS_COMPRESSED"        
    fi
    echo "⚙️ Decompressing data in csv format..."
    gzip -d "$BAN_ADDRESS_COMPRESSED"
    echo "✅ French BAN addresses has been decompressed. Get started"
fi

exit 0