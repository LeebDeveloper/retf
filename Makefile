all: compile

clean:
	@rm -rf retf
	@rm -rf retf.so

compile:
	@rustc src/retf.rs -o retf.so

retf: src/retf.rs
	@rustc --test src/retf.rs -o retf

test: retf
	@./retf