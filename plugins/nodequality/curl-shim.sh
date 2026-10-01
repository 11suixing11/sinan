#!/usr/bin/env bash
set -euo pipefail
umask 077
upload=0
for argument in "$@"; do
  [[ $argument != https://api.nodequality.com/api/v1/record ]] || upload=1
done
if [[ $upload == 0 ]]; then
  # The fixed entrypoint's rootfs download is a separate, still pending chain.
  # Every first-level source request is served locally; unknown sources fail.
  if [[ $# == 3 && $1 == '-L#o' && $2 == BenchOs.tar.gz ]]; then
    case "$3" in
      https://github.com/LloydAsp/NodeQuality/releases/download/v0.0.2/BenchOs.tar.gz|https://github.com/LloydAsp/NodeQuality/releases/download/v0.0.2/BenchOs-arm.tar.gz)
        exec "$SINAN_REAL_CURL" --connect-timeout 15 --max-time 900 "$@"
        ;;
    esac
  fi
  exec python3 "$SINAN_CHAIN_HELPER" serve "$SINAN_CHAIN_DIRECTORY" "$@"
fi
python3 "$SINAN_REPORT_HELPER" capture "$SINAN_REPORT_WORKSPACE"
if [[ ${SINAN_UPLOAD_REPORT:-false} != true ]]; then
  printf '%s\n' 'disabled' > "$SINAN_REPORT_WORKSPACE/upload-disabled.txt"
  printf '%s\n' 'Public report upload is disabled.'
  exit 0
fi
arguments=()
while [[ $# != 0 ]]; do
  if [[ $1 == --data-binary && ${2:-} == @- ]]; then
    arguments+=(--data-binary "@$SINAN_REPORT_WORKSPACE/upload.base64")
    shift 2
  else
    arguments+=("$1")
    shift
  fi
done
set +e
"$SINAN_REAL_CURL" --connect-timeout 15 --max-time 60 --max-filesize 65536 \
  --write-out $'\nSINAN_RESPONSE_STATUS:%{http_code}' "${arguments[@]}" \
  | python3 "$SINAN_REPORT_HELPER" response "$SINAN_REPORT_WORKSPACE"
statuses=("${PIPESTATUS[@]}")
set -e
[[ ${statuses[1]} == 0 ]] || exit "${statuses[1]}"
exit "${statuses[0]}"
