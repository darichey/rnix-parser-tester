.PHONY: all clean

all: reference_to_json.cpp

clean:
	rm reference_to_json

reference_to_json.cpp:
	NIX_DEBUG=1 cc reference_to_json.cpp \
		-o reference_to_json \
		-lstdc++ -lnixexpr -lnixstore -lnixutil \
		-v
