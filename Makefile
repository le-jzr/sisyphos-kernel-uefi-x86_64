build_mode := debug

target := x86_64-sisyphos-uefi
build_dir := target/$(target)/$(build_mode)
archive :=
mkisofs := xorriso -as mkisofs

CC = clang

CRATE_NAME = sisyphos-kernel-uefi-x86_64

EFI_CRT_SOURCE = src/crt0-efi-x86_64.S
EFI_CRT_OBJS = $(build_dir)/crt0-efi-x86_64.o
EFI_LDS = src/elf_x86_64_efi.lds

LDFLAGS = -nostdlib -znocombreloc -T $(EFI_LDS) -shared -Bsymbolic $(EFI_CRT_OBJS) --no-undefined

LIBNAME = sisyphos_kernel_uefi_x86_64
ARNAME = $(build_dir)/lib$(LIBNAME).a
SONAME = $(build_dir)/bootx64.so
EFINAME = $(build_dir)/bootx64.efi
ISONAME = $(build_dir)/$(CRATE_NAME).iso
HDIMAGE = $(build_dir)/$(CRATE_NAME).img
#KVM = -enable-kvm

# UEFI Firmware.
# Preset path is to OVMF shipped in ArchLinux package.
# Change to suit your setup.
BIOS = /usr/share/ovmf/ovmf_code_x64.bin

QEMUOPTS = $(KVM) -cpu max -bios $(BIOS) -no-reboot -no-shutdown -d cpu_reset,guest_errors -monitor stdio -serial none -s
OBJCOPY = objcopy
FORMAT = --target efi-app-x86_64

XARGO_ARGS = --target=$(target)

.PHONY: all cargo run

all: run

cargo:
	xargo build $(XARGO_ARGS)

clean:
	rm -rf target
	rm -f Cargo.lock

$(EFI_CRT_OBJS): $(EFI_CRT_SOURCE)
	$(CC) -c -o $@ $<

$(SONAME): $(ARNAME) $(EFI_CRT_OBJS)
	ld.lld $(LDFLAGS) -L $(build_dir) -l $(LIBNAME) -o $@

%.efi: %.so
	$(OBJCOPY) -j .text -j .sdata -j .data -j .dynamic -j .dynsym -j .rel \
		    -j .rela -j .rel.* -j .rela.* -j .rel* -j .rela* \
		    -j .reloc $(FORMAT) $*.so $@

%.efi.debug: %.so
	$(OBJCOPY) -j .debug_info -j .debug_abbrev -j .debug_aranges \
		-j .debug_line -j .debug_str -j .debug_ranges \
		-j .note.gnu.build-id $(FORMAT) $*.so $@

$(ISONAME): $(EFINAME)
	mkdir -p $(build_dir)/iso
	cp $(EFINAME) $(build_dir)/iso
	$(mkisofs) -o $@ $(build_dir)/iso

$(HDIMAGE): $(EFINAME)
	dd if=/dev/zero of=$(HDIMAGE).tmp bs=512 count=1000000
	parted $(HDIMAGE).tmp -s -a minimal mklabel gpt
	parted $(HDIMAGE).tmp -s -a minimal mkpart EFI FAT32 2048s 600000s
	parted $(HDIMAGE).tmp -s -a minimal toggle 1 boot
	dd if=/dev/zero of=$(HDIMAGE).part.img bs=512 count=600000
	mformat -i $(HDIMAGE).part.img -F -h 1 -t 1000 -n 500 -c 1
	mcopy -i $(HDIMAGE).part.img $(EFINAME) ::
	dd if=$(HDIMAGE).part.img of=$(HDIMAGE).tmp bs=512 count=550000 seek=2048 conv=notrunc
	mv $(HDIMAGE).tmp $(HDIMAGE)

run: cargo $(HDIMAGE)
	qemu-system-x86_64 $(QEMUOPTS) -drive file=$(HDIMAGE),if=ide,format=raw
	
