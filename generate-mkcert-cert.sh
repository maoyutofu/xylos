#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./generate-mkcert-cert.sh LAN_IPV4 [DNS_NAME ...]

Generate a locally trusted TLS certificate for a LAN IP address using mkcert.
The certificate also covers localhost and 127.0.0.1. Existing files are not overwritten.

Example:
  ./generate-mkcert-cert.sh 192.168.1.10
  ./generate-mkcert-cert.sh 192.168.1.10 xylos.home.arpa
EOF
}

if [[ ${1:-} == -h || ${1:-} == --help ]]; then
  usage
  exit 0
fi

if (( $# < 1 )); then
  usage >&2
  exit 2
fi

ip=$1
shift
if [[ ! $ip =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
  echo "Expected a LAN IPv4 address, such as 192.168.1.10" >&2
  exit 2
fi
IFS=. read -r -a octets <<< "$ip"
for octet in "${octets[@]}"; do
  if (( 10#$octet > 255 )); then
    echo "Invalid IPv4 address: $ip" >&2
    exit 2
  fi
done

if ! command -v mkcert >/dev/null 2>&1; then
  echo "mkcert is required. Install it first (on macOS: brew install mkcert)." >&2
  exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
output_dir="$script_dir/certs"
cert_path="$output_dir/xylos-lan-$ip-cert.pem"
key_path="$output_dir/xylos-lan-$ip-key.pem"
if [[ -e $cert_path || -L $cert_path || -e $key_path || -L $key_path ]]; then
  echo "Certificate or key already exists for $ip; existing files were left untouched." >&2
  exit 1
fi

mkdir -p "$output_dir"
umask 077
mkcert -install
mkcert -cert-file "$cert_path" -key-file "$key_path" "$ip" localhost 127.0.0.1 "$@"

ca_root=$(mkcert -CAROOT)
cat <<EOF

Add or update this block in config.toml, then restart Xylos:

[server.tls]
enabled = true
cert_path = "certs/xylos-lan-$ip-cert.pem"
key_path = "certs/xylos-lan-$ip-key.pem"

Open https://$ip:8081/app/ (adjust the port if your config differs).
Install $ca_root/rootCA.pem on each phone and enable full trust on iOS.
Never copy rootCA-key.pem to a phone or another device.
EOF
