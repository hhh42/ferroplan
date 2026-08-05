#!/usr/bin/env bash
set -euo pipefail

receipt_dir=".cloud/receipts"
mkdir -p "${receipt_dir}"
receipt="${receipt_dir}/bootstrap.json"
started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

status="ALIVE"
failed_command=""

run() {
  printf '>>> %q ' "$@"
  printf '\n'
  if ! "$@"; then
    status="BUILD_BROKEN"
    failed_command="$(printf '%q ' "$@")"
    return 1
  fi
}

if run cargo fetch --locked \
  && run cargo fmt --all --check \
  && run cargo check --locked -p ferroplan-cli --bin ferroplan-ppddl \
  && run cargo test --locked -p ferroplan ppddl::; then
  :
fi

finished="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
head="$(git rev-parse HEAD 2>/dev/null || printf UNKNOWN)"

jq -n \
  --arg schema "ferroplan-cloud-bootstrap-receipt/v1" \
  --arg state "${status}" \
  --arg started "${started}" \
  --arg finished "${finished}" \
  --arg head "${head}" \
  --arg rustc "$(rustc --version)" \
  --arg cargo "$(cargo --version)" \
  --arg failed_command "${failed_command}" \
  '{
    schema: $schema,
    final_state: $state,
    ci_used: false,
    subject: {head_sha: $head},
    toolchain: {rustc: $rustc, cargo: $cargo},
    started_at: $started,
    finished_at: $finished,
    failed_command: (if $failed_command == "" then null else $failed_command end),
    commands: [
      "cargo fetch --locked",
      "cargo fmt --all --check",
      "cargo check --locked -p ferroplan-cli --bin ferroplan-ppddl",
      "cargo test --locked -p ferroplan ppddl::"
    ]
  }' > "${receipt}"

cat "${receipt}"
[[ "${status}" == "ALIVE" ]]
