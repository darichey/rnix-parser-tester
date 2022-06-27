.PHONY: all clean

all: reference_to_json.cc

clean:
	rm reference_to_json

reference_to_json.cc:
	$(CXX) reference_to_json.cc \
		-o reference_to_json \
		-lnixexpr -lnixstore -lnixutil
