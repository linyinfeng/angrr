# shellcheck shell=bash

# References:
#   https://github.com/direnv/direnv/blob/master/stdlib.sh
#   https://github.com/direnv/direnv/blob/master/internal/cmd/rc.go

use_angrr() {
    if ! has angrr; then
        log_error "angrr: can not find angrr binary in PATH"
        return 1
    fi
    # When loading .envrc, $PWD is the project root
    runtime="$(
        angrr touch "$PWD" \
            --project \
            --log-level "${ANGRR_DIRENV_LOG:-error}" \
            --output-runtime \
            --silent
    )"
    runtime_formatted=$(LC_ALL=C printf "%.3f" "$runtime")
    log_status "angrr: touch GC roots in \"$PWD\" (took ${runtime_formatted}s)"
}

# direnvrc is loaded in the `__main__` function of direnv stdlib
# The second argument of `__main__` is the path to the RC file

# Usage: _angrr_auto_use "$@"
# Only useful in direnvrc
_angrr_auto_use() {
    # follow the same logic as source_env in stdlib
    local rc_path="$2"
    local REPLY
    if [ -d "$rc_path" ]; then
        rc_path="$rc_path/.envrc"
    fi
    realpath.dirname "$rc_path"
    local rc_path_dir="$REPLY"

    if [ ! -e "$rc_path" ]; then
        log_status "angrr: $rc_path does not exist, skip"
        return 1
    fi

    pushd "$rc_path_dir" >/dev/null || return 1
    use angrr
    popd >/dev/null || return 1
}
