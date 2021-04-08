OUT_DIR := ../../target/deploy
SOLANA_PATH := $(solana_path)
ifeq ($(SOLANA_PATH),)
SOLANA_PATH := ~/.local/share/solana/install
endif
include $(SOLANA_PATH)/active_release/bin/sdk/bpf/c/bpf.mk
