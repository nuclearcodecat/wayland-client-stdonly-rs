#!/bin/bash

reset; sudo rm -r /dev/shm/wl-*; CARGO_TERM_COLOR=always WAYTINIER_DEBUGLVL=$1 RUST_BACKTRACE=FULL cargo run --release 2>&1 | tee log
