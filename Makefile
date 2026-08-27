KERNEL := /usr/lib/modules/7.1.9-arch1-2/vmlinuz
SYSROOT := sysroot
DISKROOT := .diskroot
TARGET := x86_64-unknown-linux-musl
DISK_SIZE := 10G

.PHONY: all build base-disk pack run clean

all: run

build:
	cd rust_init && cargo build --release --target $(TARGET)
	mkdir -p $(SYSROOT)/bin
	cp rust_init/target/$(TARGET)/release/rust_init $(SYSROOT)/bin/init
	chmod +x $(SYSROOT)/bin/init

	cd sys_tools && cargo build --release --target $(TARGET)
	cp sys_tools/target/$(TARGET)/release/poweroff $(SYSROOT)/bin/poweroff
	cp sys_tools/target/$(TARGET)/release/reboot $(SYSROOT)/bin/reboot
	cp sys_tools/target/$(TARGET)/release/elfetch $(SYSROOT)/bin/elfetch
	cp sys_tools/target/$(TARGET)/release/elfpkg $(SYSROOT)/bin/elfpkg
	chmod +x $(SYSROOT)/bin/poweroff $(SYSROOT)/bin/reboot $(SYSROOT)/bin/elfetch $(SYSROOT)/bin/elfpkg

	# Applet symlinks
	ln -sf busybox $(SYSROOT)/bin/insmod
	ln -sf busybox $(SYSROOT)/bin/ip
	ln -sf busybox $(SYSROOT)/bin/sh
	ln -sf busybox $(SYSROOT)/bin/udhcpc

	# DRM/KMS support for framebuffer and future Wayland compositors
	mkdir -p $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/virtio
	mkdir -p $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm
	mkdir -p $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/virtio

	zstd -dc /usr/lib/modules/7.1.9-arch1-2/kernel/drivers/virtio/virtio_dma_buf.ko.zst > $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/virtio/virtio_dma_buf.ko 2>/dev/null || true
	zstd -dc /usr/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/tiny/simpledrm.ko.zst > $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/simpledrm.ko 2>/dev/null || true
	zstd -dc /usr/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/virtio/virtio-gpu.ko.zst > $(SYSROOT)/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/virtio/virtio-gpu.ko 2>/dev/null || true

	# Mount points for package-provided shared libraries
	mkdir -p $(SYSROOT)/lib64 $(SYSROOT)/usr/lib $(SYSROOT)/etc
	cp /home/ben/elfux_package_repo.json $(SYSROOT)/etc/elfux_package_repo.json

base-disk:
	rm -rf $(DISKROOT)
	mkdir -p $(DISKROOT)
	chmod +x util/install-base-system.sh
	util/install-base-system.sh $(DISKROOT) /home/ben/elfux_package_repo.json base-system
	qemu-img create -f raw elfux_disk.raw $(DISK_SIZE)
	mkfs.ext4 -F -d $(DISKROOT) elfux_disk.raw
	if [ -f elfux_disk.qcow2 ]; then cp elfux_disk.qcow2 elfux_disk.qcow2.bak; fi
	qemu-img convert -f raw -O qcow2 elfux_disk.raw elfux_disk.qcow2
	rm -f elfux_disk.raw

pack: build
	cd $(SYSROOT) && find . -print0 | cpio --null -o --format=newc 2>/dev/null | gzip -1 > ../initramfs.cpio.gz

run: pack
	qemu-system-x86_64 \
		-m 2G \
		-kernel $(KERNEL) \
		-initrd initramfs.cpio.gz \
		-drive file=elfux_disk.qcow2,format=qcow2,id=hd0,if=none \
		-device virtio-blk-pci,drive=hd0 \
		-device virtio-vga \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0 \
		-append "console=tty0 rdinit=/bin/init -quiet" 

clean: 
	cd rust_init && cargo clean
	cd sys_tools && cargo clean
	rm -f initramfs.cpio.gz