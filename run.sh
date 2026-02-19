#!/bin/bash

if [[ "$1" == "s" ]]; then
  reset; sudo rm -r /dev/shm/wl-*; CARGO_TERM_COLOR=always WAYTINIER_DEBUGLVL=$2 RUST_BACKTRACE=FULL strace cargo run --example new --release 2>&1 | tee log
else
  reset; sudo rm -r /dev/shm/wl-*; CARGO_TERM_COLOR=always WAYTINIER_DEBUGLVL=$1 RUST_BACKTRACE=FULL cargo run --example new --release 2>&1 | tee log
fi
