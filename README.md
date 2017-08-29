# sisyphos-kernel-uefi-x86_64
A Rust kernel running on bare UEFI (no separate bootloader). Very early stage.

Check the [wiki](https://github.com/le-jzr/sisyphos-kernel-uefi-x86_64/wiki) for some notes about this thing.

Basically, the goal is to build a non-opinionated microkernel that can load regular ELF64 programs as kernel "modules".
Actually, just fairly conventional processes, except running in kernel space (they are assumed to be written in Rust
and [reproducible](https://reproducible-builds.org/), so that hardware protections are unnecessary, similar but
unrelated to Microsoft's [Singularity](https://en.wikipedia.org/wiki/Singularity_(operating_system)) project).

The core micro/nano/whateverkernel will link up the loaded applications with a builtin dynamically linked library that
exposes its functionality, moving the responsibility for higher-level problems (such as syscalls) into these loadable
binaries, and also allowing simple emulation without virtualization for debugging purposes.
