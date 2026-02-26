i don't plan on continuing the development. the goal was to learn wayland's internals which i reached.  

waytinier is a tiny experimental wayland client library for rust.  
it was originally supposed to be dependant on nothing but std and libc (which std also depends on).  
i've allowed myself to use _libloading_ to load _libgbm_, because it's a tiny crate and doesn't increase build times drastically.  

the _machine_ example can (at the time of testing) open a window and draw an image while weighing a bit over 650KiB (with default cargo/rustc settings)  
with these cargo optimizations for size enabled, i managed to get the binary size down to ~380KiB:  
 - elf stripping enabled
 - opt level set to »z«
 - lto set to »fat«
 - only one codegen unit enabled
 - panic unwinding disabled

interestingly, setting the opt-level to »s« **increased** the "raw" binary's size by 4KiB and »z« increased it by **50KiB**!  
these settings increased the build times from ~0.7s to ~3.6s  

waytinier currently offers window (xdg_toplevel) creation and drawing on shared memory buffers. i've also added an option to use dma buf fd's but am not willing to test whether that works.  
documentation is highly lacking, as in, there is none. i may get to that one day  

the _WAYTINIER_DEBUGLEVEL_ environment variable can be set to values from -1 to 4 to change the amount of logs emmited. a _nolog_ feature is available to disable logging completely  

see the examples dir for examples. _machine_ and _new_ should both work.  

building waytinier **requires** the nightly rust compiler toolchain.  
