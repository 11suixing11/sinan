#!/usr/bin/env bash
# Translate the pinned loader's fixed trace binary request for ARM64 hosts.
set -euo pipefail
fixed_command='wget https://github.com/nxtrace/NTrace-core/releases/download/v1.3.7/nexttrace_linux_amd64 -qO /usr/local/bin/nexttrace'
if [[ $# == 4 && $2 == /bin/bash && $3 == -c && $4 == "$fixed_command" ]]; then
  case "$(uname -m)" in
    aarch64|arm64)
      exec "$SINAN_REAL_CHROOT" "$1" "$2" "$3" \
        'wget https://github.com/nxtrace/NTrace-core/releases/download/v1.3.7/nexttrace_linux_arm64 -qO /usr/local/bin/nexttrace'
      ;;
  esac
fi
exec "$SINAN_REAL_CHROOT" "$@"
