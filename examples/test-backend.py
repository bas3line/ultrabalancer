#!/usr/bin/env python3
"""Simple HTTP backend server for testing UltraBalancer"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import sys
import socket

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        port = self.server.server_address[1]
        self.wfile.write(f'Backend server on port {port}\n'.encode())
    
    def do_HEAD(self):
        self.send_response(200)
        self.end_headers()
    
    def log_message(self, format, *args):
        sys.stdout.write(f"[{self.server.server_address[1]}] {format % args}\n")

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8001
    server = HTTPServer(('0.0.0.0', port), Handler)
    print(f'Backend server listening on port {port}')
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print('\nShutting down...')
        server.shutdown()
