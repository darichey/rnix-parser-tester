.PHONY: all nix clean

all: reference_to_json.cc

nix:
	$(MAKE) -C nix -j $(NIX_BUILD_CORES)

clean:
	$(MAKE) -C nix clean
	rm reference_to_json

reference_to_json.cc: nix
	$(CXX) reference_to_json.cc \
		-o reference_to_json \
		-include nix/config.h \
		-I nix/src/libutil -I nix/src/libexpr -I nix/src/libstore \
		-L nix/outputs/out/lib \
		-lnixexpr -lnixstore -lnixutil
