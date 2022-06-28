.PHONY: all clean

all: lib

lib: libreference_to_json.a(reference_to_json.o)

clean:
	-rm libreference_to_json.a

example: lib
	$(CC) example.c -o example libreference_to_json.a -lnixexpr -lnixutil -lnixstore -lstdc++