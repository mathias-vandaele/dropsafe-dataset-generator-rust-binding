#!/bin/sh
set -e

# Variables de configuration
PBF_URL="https://download.geofabrik.de/europe/france-latest.osm.pbf"
PBF_FILE="/data/mld/france-latest.osm.pbf"
OSRM_FILE_CHECK="/data/mld/france-latest.osrm.timestamp"
OSRM_FILE="/data/mld/france-latest.osrm"
apk update && apk add curl gzip

# Vérifie si le fichier .osrm final existe déjà
if [ -f "$OSRM_FILE_CHECK" ]; then
    echo "✅ OSRM data already exists. Quick start."
else
    echo "⏳ OSRM data not found. Starting import process..."
    echo `ls /data/`
    # Téléchargement du fichier PBF s'il n'existe pas
    if [ ! -f "$PBF_FILE" ]; then
        echo "📥 Downloading $PBF_URL..."
        curl -L "$PBF_URL" -o "$PBF_FILE"
    fi

    echo "⚙️ Extracting data in OSRM format..."
    osrm-extract -p /opt/car.lua "$PBF_FILE"
    echo "🎉 Extraction completed!"

    echo "⚙️ Processing MLD..."
    osrm-partition "$OSRM_FILE"
    osrm-customize "$OSRM_FILE"
    echo "🎉 MLD processing completed!"

fi

exit 0