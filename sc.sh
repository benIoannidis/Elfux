KVER="7.1.9-arch1-2"

# 1. Create target directories on sysroot for mountpoints/symlinks
mkdir -p sysroot/lib64

# 2. Copy the dynamic linker directly into sysroot/lib64 (it's small, ~160KB)
cp -P /lib64/ld-linux-x86-64.so.2 sysroot/lib64/ 2>/dev/null || cp -P /usr/lib/ld-linux-x86-64.so.2 sysroot/lib64/

# 3. Copy host glibc libraries to persistent storage mount target (sysroot/lib is symlinked or mounted to /mnt/lib)
# Alternatively, copy needed glibc libs directly into sysroot/lib:
cp -P /usr/lib/libm.so.6 sysroot/lib/
cp -P /usr/lib/libc.so.6 sysroot/lib/
cp -P /usr/lib/libpthread.so.0 sysroot/lib/
cp -P /usr/lib/libdl.so.2 sysroot/lib/
