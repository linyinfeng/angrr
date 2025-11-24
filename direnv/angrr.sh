# shellcheck shell=bash

use_angrr() {
    layout_dir="$(direnv_layout_dir)"
    if ! has angrr; then
        log_error "angrr: can not find angrr binary in PATH"
        return 1
    fi
    log_status "angrr: touch GC roots $layout_dir"
    RUST_LOG="${ANGRR_DIRENV_LOG:-angrr=error}" angrr touch "$layout_dir" --slient
}
