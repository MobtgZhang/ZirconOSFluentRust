# ZirconOSFluentRust — cargo + pack-esp.sh + QEMU/OVMF (x86_64)
#
#   make / make help     — list targets
#   make build           — Debug + Release ESP trees under build/esp-*
#   make build-debug     — debug ZBM10 + kernel → build/esp-debug
#   make build-release   — release artifacts → build/esp-release
#   make run             — same as run-debug (VGA + COM1→stdio, kernel log on this terminal)
#   make run-debug       — QEMU with display; serial stdio (kernel debug_write / early log)
#   make run-release     — headless (-display none); serial stdio
#
# Override: OVMF_CODE=... QEMU_EXTRA='-m 512M' make run-debug

ROOT := $(abspath .)
PACK := $(ROOT)/scripts/pack-esp.sh
RUNQ := $(ROOT)/scripts/run-qemu-x86_64.sh

ESP_DEBUG := $(ROOT)/build/esp-debug
ESP_RELEASE := $(ROOT)/build/esp-release

QEMU_EXTRA ?=

.PHONY: default help build build-debug build-release run run-debug run-release clean

default: help

help:
	@echo "ZirconOSFluentRust"
	@echo "  make build              — build-debug + build-release (ESP under build/)"
	@echo "  make build-debug        — PROFILE=debug:  ZBM10 + NT10KRNL.BIN → $(ESP_DEBUG)"
	@echo "  make build-release      — PROFILE=release → $(ESP_RELEASE)"
	@echo "  make run / run-debug    — QEMU+OVMF, VGA + serial on stdio (kernel log in this terminal)"
	@echo "  make run-release        — same firmware/kernel (release), headless"
	@echo "  make clean              — remove build/esp-debug and build/esp-release"

build: build-debug build-release

build-debug:
	@mkdir -p "$(ESP_DEBUG)"
	cd "$(ROOT)" && PROFILE=debug "$(PACK)" "$(ESP_DEBUG)"

build-release:
	@mkdir -p "$(ESP_RELEASE)"
	cd "$(ROOT)" && PROFILE=release "$(PACK)" "$(ESP_RELEASE)"

run: run-debug

run-debug: build-debug
	ZBM10_ESP="$(ESP_DEBUG)" "$(RUNQ)" $(QEMU_EXTRA)

run-release: build-release
	ZBM10_ESP="$(ESP_RELEASE)" "$(RUNQ)" -display none $(QEMU_EXTRA)

clean:
	rm -rf "$(ESP_DEBUG)" "$(ESP_RELEASE)"
