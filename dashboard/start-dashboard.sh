#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

case "$1" in
    start)
        echo "Starting UltraBalancer Dashboard..."
        docker compose up -d --build
        echo "Dashboard started successfully!"
        echo "Grafana: http://localhost:3000"
        echo "Prometheus: http://localhost:9090"
        ;;
    stop)
        echo "Stopping UltraBalancer Dashboard..."
        docker compose down -v
        docker network rm ultrabalancer-net 2>/dev/null || true
        echo "Dashboard stopped."
        ;;
    restart)
        echo "Restarting UltraBalancer Dashboard..."
        docker compose restart
        echo "Dashboard restarted."
        ;;
    status)
        echo "Checking dashboard status..."
        docker ps --filter "name=ultrabalancer-dashboard"
        ;;
    logs)
        docker compose logs -f "${2:-}"
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|logs}"
        exit 1
        ;;
esac
