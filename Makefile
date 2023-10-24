docs/pkg/aoc_lang_bg.wasm: src/*.rs
	echo "Building wasm..." $@ $<
	wasm-pack build --target no-modules --out-dir docs/pkg

serve: docs/pkg/aoc_lang_bg.wasm
	python3 -m http.server -d docs
