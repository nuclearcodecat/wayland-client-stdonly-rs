waytinier is a tiny experimental wayland client library for rust.  
it was originally supposed to be dependant on nothing but std and libc (which std also depends on).  
right now i've allowed myself to use _libloading_ to load _libgbm_, because it's a tiny crate and doesn't increase build times drastically. i may remove it later anyway for educational purposes.  
i will probably definitely absolutely 99.9%ly not release waytinier on crates  

waytinier is tiny by design. the _machine_ example can (at the time of testing) open a window and draw an image while weighing a bit over 650KiB (with default cargo/rustc settings)  
with these cargo optimizations for size enabled, i managed to get the binary size down to ~380KiB:  
 - elf stripping enabled
 - opt level set to »z«
 - lto set to »fat«
 - only one codegen unit enabled
 - panic unwinding disabled

interestingly, setting the opt-level to »s« **increased** the "raw" binary's size by 4KiB and »z« increased it by **50KiB**!  
these settings increased the build times from ~0.7s to ~3.6s  

waytinier currently offers window (xdg_toplevel) creation and drawing on shared memory buffers. i'm constantly fighthing to get _zwp_linux_dmabuf_v1_ working. unfortunately i'm kind of burned out right now after a rewrite of a large portion of the code.  
documentation is highly lacking, as in, there is none. i may get to that one day  

the _WAYTINIER_DEBUGLEVEL_ environment variable can be set to values from 0 to 4 to change the amount of logs emmited. a _nolog_ feature is available to disable logging completely  

future plans include keyboard and/or mouse support and dmabuf working

see the examples dir for a simple example.  

building waytinier **requires** the nightly rust compiler toolchain.  
the most important feature used (among others) is *unix_socket_ancillary_data*, which allows me to attach file descriptors to wayland requests  
