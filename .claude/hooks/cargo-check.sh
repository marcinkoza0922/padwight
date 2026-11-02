#!/bin/sh
# After an edit to a Rust file, runs `cargo check` and reports only failures.
# Checks the checkout the edited file is in, so agents in git worktrees get their own tree checked;
# worktrees build into the main checkout's target folder, sharing its compiled dependencies.
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in *.rs) ;; *) exit 0 ;; esac
root=$(cd "$(dirname "$file")" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null) || root=$CLAUDE_PROJECT_DIR
cd "$root" || exit 0
if [ "$root" != "$CLAUDE_PROJECT_DIR" ] && [ -z "$CARGO_TARGET_DIR" ]; then
    export CARGO_TARGET_DIR="$CLAUDE_PROJECT_DIR/target"
fi
out=$(cargo check --all-targets --message-format=short 2>&1) && exit 0
echo "$out" | grep -E '^[^ ]+: (error|warning)' | head -20 >&2
exit 2
