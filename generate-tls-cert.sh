#!/usr/bin/env sh
set -eu

usage() {
  cat <<'EOF'
Usage: ./generate-tls-cert.sh [options]

Generate a local self-signed TLS certificate for xylos.

Options:
  -d, --dir DIR       Output directory (default: certs)
  -n, --name NAME     Certificate common name and DNS SAN (default: localhost)
  -i, --ip IP         IP SAN (default: 127.0.0.1)
  -t, --days DAYS     Validity in days (default: 365)
  -h, --help          Show this help

Example:
  ./generate-tls-cert.sh
  ./generate-tls-cert.sh --name xylos.local --ip 192.168.1.10 --days 825

Config:
  [server.tls]
  enabled = true
  cert_path = "certs/xylos-cert.pem"
  key_path = "certs/xylos-key.pem"
EOF
}

output_dir="certs"
name="localhost"
ip="127.0.0.1"
days="365"

while [ "$#" -gt 0 ]; do
  case "$1" in
    -d|--dir)
      output_dir="${2:?missing value for $1}"
      shift 2
      ;;
    -n|--name)
      name="${2:?missing value for $1}"
      shift 2
      ;;
    -i|--ip)
      ip="${2:?missing value for $1}"
      shift 2
      ;;
    -t|--days)
      days="${2:?missing value for $1}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required but was not found in PATH" >&2
  exit 1
fi

case "$days" in
  ''|*[!0-9]*)
    echo "--days must be a positive integer" >&2
    exit 2
    ;;
esac

if [ "$days" -eq 0 ]; then
  echo "--days must be greater than 0" >&2
  exit 2
fi

mkdir -p "$output_dir"

cert_path="$output_dir/xylos-cert.pem"
key_path="$output_dir/xylos-key.pem"
config_path="$output_dir/xylos-openssl.cnf"

cat >"$config_path" <<EOF
[req]
default_bits = 2048
prompt = no
default_md = sha256
distinguished_name = dn
x509_extensions = v3_req

[dn]
CN = $name

[v3_req]
subjectAltName = @alt_names
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth

[alt_names]
DNS.1 = $name
IP.1 = $ip
EOF

openssl req \
  -x509 \
  -newkey rsa:2048 \
  -nodes \
  -days "$days" \
  -keyout "$key_path" \
  -out "$cert_path" \
  -config "$config_path" \
  >/dev/null 2>&1

chmod 600 "$key_path"
chmod 644 "$cert_path"

cat <<EOF
Generated TLS certificate:
  cert_path = "$cert_path"
  key_path = "$key_path"

Add this to config.toml:

[server.tls]
enabled = true
cert_path = "$cert_path"
key_path = "$key_path"
EOF
