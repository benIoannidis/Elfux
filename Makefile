KERNEL := /usr/lib/modules/7.1.9-arch1-2/vmlinuz
SYSROOT := sysroot
TARGET := x86_64-unknown-linux-musl

.PHONY: all build pack run clean

all: run

build:
	# build PID 1 init
	cd rust_init && cargo build --release --target $(TARGET)
	mkdir -p $(SYSROOT)/bin
	cp rust_init/target/$(TARGET)/release/rust_init $(SYSROOT)/bin/init
	chmod +x $(SYSROOT)/bin/init

	#build pwrmgmt tools
	cd sys_tools && cargo build --release --target $(TARGET)
	cp sys_tools/target/$(TARGET)/release/poweroff $(SYSROOT)/bin/poweroff
	cp sys_tools/target/$(TARGET)/release/reboot $(SYSROOT)/bin/reboot
	cp sys_tools/target/$(TARGET)/release/elfetch $(SYSROOT)/bin/elfetch
	cp sys_tools/target/$(TARGET)/release/elfpkg $(SYSROOT)/bin/elfpkg
	chmod +x $(SYSROOT)/sbin/poweroff $(SYSROOT)/bin/reboot $(SYSROOT)/bin/elfetch $(SYSROOT)/bin/elfpkg

pack: build
	cd $(SYSROOT) && find . -print0 | cpio --null -ov --format=newc | gzip -9 > ../initramfs.cpio.gz

run: pack
	qemu-system-x86_64 \
		-kernel $(KERNEL) \
		-initrd initramfs.cpio.gz \
		-drive file=elfux_disk.qcow2,format=qcow2,id=hd0,if=none \
		-device virtio-blk-pci,drive=hd0 \
		-append "console=ttyS0 rdinit=/sbin/init -quiet" \
		-nographic

clean: 
	cd rust_init && cargo clean
	cd sys_tools && cargo clean
	rm -f initramfs.cpio.gz